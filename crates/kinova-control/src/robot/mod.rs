//! Native Kortex TCP client for Kinova Gen3/Gen3 Lite robots.
//!
//! Implements the request/response protocol used on the Kortex TCP transport
//! (default port 10000): length-prefixed protobuf [`Frame`]s, session
//! management, and per-request correlation via the header message id.

pub mod calls;
pub mod router;
pub mod transport;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use prost::Message;
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::oneshot;
use tokio::time::timeout;

use crate::api::Frame;
use crate::api::base::ActuatorInformation;
use crate::api::common::Empty;
use crate::api::session::CreateSessionInfo;
use crate::config::Config;

use router::{MSG_FRAME_RESPONSE, RobotError, SERVICE_VERSION, frame_type, message_id, session_id};
use transport::{read_frame, request_frame, write_frame};

const SESSION_UID_CREATE: u32 = 0x10001;
const BASE_UID_GET_ACTUATOR_COUNT: u32 = 0x200ab;

type PendingMap = Arc<Mutex<HashMap<u16, oneshot::Sender<Result<Frame, RobotError>>>>>;

/// Shared handle to the robot. Wrap in `Arc<Mutex<RobotClient>>` for
/// single-flight access from the web layer.
pub struct RobotClient {
    cfg: Config,
    alive: Arc<AtomicBool>,
    pending: PendingMap,
    writer: Option<OwnedWriteHalf>,
    session_id: u32,
    next_msg_id: u16,
    rpc_timeout: Duration,
    actuator_count: u32,
    reconnect_failures: u32,
    next_reconnect_at: Option<Instant>,
    reader: Option<tokio::task::JoinHandle<()>>,
}

impl RobotClient {
    /// Bound individual RPC waits; motion supervisors must not wait ten seconds
    /// behind a missing telemetry response before attempting Stop.
    pub fn set_rpc_timeout(&mut self, duration: Duration) {
        self.rpc_timeout = duration;
    }

    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            alive: Arc::new(AtomicBool::new(false)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            writer: None,
            session_id: 0,
            next_msg_id: 0,
            rpc_timeout: Duration::from_secs(10),
            actuator_count: 0,
            reconnect_failures: 0,
            next_reconnect_at: None,
            reader: None,
        }
    }

    /// Connects (or reconnects) to the robot and opens a session.
    ///
    /// Failed attempts are spaced by an exponential backoff so periodic callers
    /// (telemetry polls) do not hammer an absent robot. While backed off this
    /// returns `Err` immediately without touching the network.
    pub async fn ensure_connected(&mut self) -> Result<(), RobotError> {
        if self.alive.load(Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(next) = self.next_reconnect_at {
            let now = Instant::now();
            if now < next {
                return Err(RobotError::Other(format!(
                    "robot offline; next reconnect attempt in {:.1}s",
                    next.saturating_duration_since(now).as_secs_f32()
                )));
            }
        }
        tracing::info!(
            "connecting to robot at {}:{}",
            self.cfg.robot_ip,
            self.cfg.robot_port
        );
        match self.connect_inner().await {
            Ok(()) => {
                self.reconnect_failures = 0;
                self.next_reconnect_at = None;
                Ok(())
            }
            Err(e) => {
                self.alive.store(false, Ordering::SeqCst);
                self.writer = None;
                if let Some(reader) = self.reader.take() {
                    reader.abort();
                }
                fail_all_pending(&self.pending, "connection setup failed");
                self.reconnect_failures = self.reconnect_failures.saturating_add(1);
                let delay = backoff_delay(
                    self.cfg.reconnect_base_ms,
                    self.cfg.reconnect_max_ms,
                    self.reconnect_failures,
                );
                self.next_reconnect_at = Some(Instant::now() + delay);
                tracing::warn!("{e}; retrying in {delay:?}");
                Err(e)
            }
        }
    }

    /// Remaining backoff delay before the next connection attempt, if any.
    pub fn reconnect_delay(&self) -> Option<Duration> {
        let next = self.next_reconnect_at?;
        Some(next.saturating_duration_since(Instant::now()))
    }

    /// Number of arm actuators, cached at connect time.
    pub fn actuator_count(&self) -> u32 {
        self.actuator_count
    }

    /// Whether a transport session is currently active.
    pub fn is_connected(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    async fn connect_inner(&mut self) -> Result<(), RobotError> {
        let addr = (self.cfg.robot_ip.as_str(), self.cfg.robot_port);
        let target = format!("{}:{}", self.cfg.robot_ip, self.cfg.robot_port);
        let connect_timeout = Duration::from_millis(self.cfg.connect_timeout_ms);
        let stream = match timeout(connect_timeout, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                return Err(RobotError::Other(format!(
                    "connect to {target} failed: {e}"
                )));
            }
            Err(_) => return Err(RobotError::Other(format!("connect to {target} timed out"))),
        };
        let _ = stream.set_nodelay(true);

        let (reader, writer) = stream.into_split();
        // An old reader must not mark a replacement session dead.
        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
        self.alive.store(false, Ordering::SeqCst);
        self.alive = Arc::new(AtomicBool::new(true));
        self.pending = Arc::new(Mutex::new(HashMap::new()));
        self.writer = Some(writer);
        self.session_id = 0;
        self.alive.store(true, Ordering::SeqCst);

        let pending = self.pending.clone();
        let alive = self.alive.clone();
        self.reader = Some(tokio::spawn(reader_loop(reader, pending, alive)));

        if let Err(e) = self.create_session().await {
            self.alive.store(false, Ordering::SeqCst);
            self.writer = None;
            return Err(e);
        }

        let count = self
            .call_response::<ActuatorInformation>(BASE_UID_GET_ACTUATOR_COUNT, None::<Empty>)
            .await?
            .count;
        self.actuator_count = count;
        tracing::info!("session established, {count} actuators");
        Ok(())
    }

    async fn create_session(&mut self) -> Result<(), RobotError> {
        let info = CreateSessionInfo {
            username: self.cfg.username.clone(),
            session_inactivity_timeout: self.cfg.session_inactivity_timeout_ms,
            password: self.cfg.password.clone(),
            connection_inactivity_timeout: self.cfg.connection_inactivity_timeout_ms,
        };
        let mut payload = Vec::new();
        info.encode(&mut payload)?;

        let resp = self.call(SESSION_UID_CREATE, Some(payload)).await?;
        let header = resp
            .header
            .ok_or(RobotError::Session("missing response header"))?;
        self.session_id = session_id(&header);
        if self.session_id == 0 {
            return Err(RobotError::Session("robot returned session id 0"));
        }
        Ok(())
    }

    /// Sends a request for the given service function and awaits its response.
    async fn call(
        &mut self,
        service_uid: u32,
        payload: Option<Vec<u8>>,
    ) -> Result<Frame, RobotError> {
        if !self.alive.load(Ordering::SeqCst) {
            return Err(RobotError::NotConnected);
        }

        self.next_msg_id = self.next_msg_id.wrapping_add(1);
        let msg_id = self.next_msg_id;

        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(msg_id, tx);

        let frame = request_frame(
            SERVICE_VERSION,
            service_uid,
            self.session_id,
            msg_id,
            payload,
        );
        let writer = self.writer.as_mut().expect("writer present while alive");
        let write = timeout(self.rpc_timeout, write_frame(writer, &frame)).await;
        if !matches!(write, Ok(Ok(()))) {
            self.alive.store(false, Ordering::SeqCst);
            self.pending.lock().unwrap().remove(&msg_id);
            return Err(match write {
                Ok(Err(e)) => RobotError::Frame(e),
                _ => RobotError::Timeout,
            });
        }

        match timeout(self.rpc_timeout, rx).await {
            Ok(Ok(Ok(frame))) => {
                if let Some(err) = router::decode_response_error(&frame) {
                    return Err(err);
                }
                Ok(frame)
            }
            Ok(Err(_)) | Ok(Ok(Err(_))) => {
                self.alive.store(false, Ordering::SeqCst);
                Err(RobotError::ReaderGone)
            }
            Err(_) => {
                self.pending.lock().unwrap().remove(&msg_id);
                Err(RobotError::Timeout)
            }
        }
    }

    async fn call_empty(&mut self, service_uid: u32) -> Result<(), RobotError> {
        self.call(service_uid, None).await.map(|_| ())
    }

    async fn call_response<T: Message + Default>(
        &mut self,
        service_uid: u32,
        request: Option<impl Message>,
    ) -> Result<T, RobotError> {
        let payload = request.map(|req| {
            let mut buf = Vec::new();
            req.encode(&mut buf).expect("encode request payload");
            buf
        });
        let frame = self.call(service_uid, payload).await?;
        T::decode(frame.payload.as_slice()).map_err(RobotError::Decode)
    }
}

impl Drop for RobotClient {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
    }
}

/// Reads frames from the transport and resolves pending request futures.
async fn reader_loop(mut reader: OwnedReadHalf, pending: PendingMap, alive: Arc<AtomicBool>) {
    loop {
        let frame = match read_frame(&mut reader).await {
            Ok(frame) => frame,
            Err(e) => {
                tracing::warn!("kortex transport closed: {e}");
                alive.store(false, Ordering::SeqCst);
                fail_all_pending(&pending, &e.to_string());
                return;
            }
        };

        let Some(header) = frame.header else { continue };
        if frame_type(&header) != MSG_FRAME_RESPONSE {
            continue; // notifications and pings are not subscribed to
        }

        let id = message_id(&header);
        if let Some(tx) = pending.lock().unwrap().remove(&id) {
            let _ = tx.send(Ok(frame));
        }
    }
}

fn fail_all_pending(pending: &PendingMap, reason: &str) {
    let mut guard = pending.lock().unwrap();
    for (_, tx) in guard.drain() {
        let _ = tx.send(Err(RobotError::Other(format!("connection lost: {reason}"))));
    }
}

/// `base * 2^(failures-1)` capped at `max`; `failures` counts consecutive
/// failed attempts and is at least 1 when a delay is scheduled.
fn backoff_delay(base_ms: u64, max_ms: u64, failures: u32) -> Duration {
    let shift = failures.saturating_sub(1).min(63);
    let factor = 1u64 << shift;
    let scaled = base_ms.max(1).saturating_mul(factor);
    Duration::from_millis(scaled.min(max_ms.max(1)))
}

#[cfg(test)]
mod tests {
    use super::backoff_delay;
    use std::time::Duration;

    #[test]
    fn backoff_grows_exponentially_and_caps() {
        let ms = |f| backoff_delay(1_000, 30_000, f).as_millis();
        assert_eq!(ms(1), 1_000);
        assert_eq!(ms(2), 2_000);
        assert_eq!(ms(3), 4_000);
        assert_eq!(ms(4), 8_000);
        assert_eq!(ms(5), 16_000);
        assert_eq!(ms(6), 30_000);
        assert_eq!(ms(100), 30_000);
    }

    #[test]
    fn backoff_never_overflows_or_returns_zero() {
        assert!(backoff_delay(u64::MAX, 30_000, 1) <= Duration::from_millis(30_000));
        assert_eq!(backoff_delay(0, 0, 1), Duration::from_millis(1));
    }
}
