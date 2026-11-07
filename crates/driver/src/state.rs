use crate::config::Config;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub fn epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Why a motion device last stopped. Persists until the next explicit enable.
#[derive(Clone, Debug, Serialize)]
pub struct StopInfo {
    pub reason: String,
    pub at: f64,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub config_path: Arc<String>,
    pub status: Arc<Mutex<Status>>,
    pub drive: Arc<Mutex<roboteq::Drive>>,
    pub drive_generation: Arc<AtomicU64>,
    pub arm_enabled: Arc<AtomicBool>,
    pub arm_stop: Arc<AtomicU64>,
    pub arm_jog: Arc<Mutex<Option<(crate::arm::Jog, Instant)>>>,
    pub arm_seen: Arc<Mutex<Instant>>,
    pub arm_tx: tokio::sync::mpsc::Sender<crate::arm::Request>,
    pub drive_stop_info: Arc<Mutex<Option<StopInfo>>>,
    pub arm_stop_info: Arc<Mutex<Option<StopInfo>>>,
    pub zed_reset: Arc<AtomicBool>,
    pub zed_generate_mesh: Arc<AtomicBool>,
    pub zed_mesh: Arc<Mutex<zed::MeshSnapshot>>,
    pub yaw_feedback: Arc<Mutex<Option<roboteq::YawFeedback>>>,
    pub images: Arc<Mutex<[Vec<u8>; 2]>>,
    pub oak_images: Arc<Mutex<oakd::Images>>,
    pub d435: d435::Camera,
    pub simulate: bool,
    pub shutting_down: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(
        config: Config,
        path: String,
        simulate: bool,
    ) -> (Self, tokio::sync::mpsc::Receiver<crate::arm::Request>) {
        let (arm_tx, arm_rx) = tokio::sync::mpsc::channel(8);
        (
            Self {
                config: Arc::new(Mutex::new(config)),
                config_path: Arc::new(path),
                status: Default::default(),
                drive: Default::default(),
                drive_generation: Arc::new(AtomicU64::new(0)),
                arm_enabled: Arc::new(AtomicBool::new(false)),
                arm_stop: Arc::new(AtomicU64::new(0)),
                arm_jog: Default::default(),
                arm_seen: Arc::new(Mutex::new(Instant::now())),
                arm_tx,
                drive_stop_info: Default::default(),
                arm_stop_info: Default::default(),
                zed_reset: Arc::new(AtomicBool::new(false)),
                zed_generate_mesh: Arc::new(AtomicBool::new(false)),
                zed_mesh: Default::default(),
                yaw_feedback: Default::default(),
                images: Default::default(),
                oak_images: Default::default(),
                d435: Default::default(),
                simulate,
                shutting_down: Arc::new(AtomicBool::new(false)),
            },
            arm_rx,
        )
    }
    pub fn record_drive_stop(&self, reason: impl Into<String>) {
        let reason = reason.into();
        eprintln!("drive stopped: {reason}");
        *self.drive_stop_info.lock().unwrap() = Some(StopInfo {
            reason,
            at: epoch(),
        });
    }
    pub fn record_arm_stop(&self, reason: impl Into<String>) {
        let reason = reason.into();
        eprintln!("arm stopped: {reason}");
        *self.arm_stop_info.lock().unwrap() = Some(StopInfo {
            reason,
            at: epoch(),
        });
    }
    pub fn clear_drive_stop(&self) {
        *self.drive_stop_info.lock().unwrap() = None;
    }
    pub fn clear_arm_stop(&self) {
        *self.arm_stop_info.lock().unwrap() = None;
    }
    #[allow(dead_code)]
    pub fn stop_drive(&self) {
        self.stop_drive_with("stopped");
    }
    pub fn stop_drive_with(&self, reason: impl Into<String>) {
        let was_armed = {
            let mut drive = self.drive.lock().unwrap();
            let was_armed = drive.is_armed();
            drive.stop();
            was_armed
        };
        self.drive_generation.fetch_add(1, Ordering::SeqCst);
        if was_armed {
            self.record_drive_stop(reason);
        }
    }
    /// Stop without replacing an already-recorded, more specific reason.
    pub fn stop_drive_preserving_reason(&self, reason: impl Into<String>) {
        let (was_armed, internal_reason) = {
            let mut drive = self.drive.lock().unwrap();
            let was_armed = drive.is_armed();
            let internal_reason = drive.take_stop_reason();
            drive.stop();
            (was_armed, internal_reason)
        };
        self.drive_generation.fetch_add(1, Ordering::SeqCst);
        if was_armed {
            let mut stop = self.drive_stop_info.lock().unwrap();
            if stop.is_none() {
                let reason = internal_reason.unwrap_or_else(|| reason.into());
                eprintln!("drive stopped: {reason}");
                *stop = Some(StopInfo {
                    reason,
                    at: epoch(),
                });
            }
        }
    }
    #[allow(dead_code)]
    pub fn stop_arm(&self) {
        self.stop_arm_with("stopped");
    }
    pub fn stop_arm_with(&self, reason: impl Into<String>) {
        let was_armed = self.arm_enabled.load(Ordering::SeqCst);
        {
            let mut jog = self.arm_jog.lock().unwrap();
            self.arm_enabled.store(false, Ordering::SeqCst);
            self.arm_stop.fetch_add(1, Ordering::SeqCst);
            *jog = None;
        }
        if was_armed {
            self.record_arm_stop(reason);
        }
    }
    /// Stop without replacing an already-recorded, more specific reason.
    pub fn stop_arm_preserving_reason(&self, reason: impl Into<String>) {
        let was_armed = self.arm_enabled.load(Ordering::SeqCst);
        {
            let mut jog = self.arm_jog.lock().unwrap();
            self.arm_enabled.store(false, Ordering::SeqCst);
            self.arm_stop.fetch_add(1, Ordering::SeqCst);
            *jog = None;
        }
        if was_armed {
            let mut stop = self.arm_stop_info.lock().unwrap();
            if stop.is_none() {
                let reason = reason.into();
                eprintln!("arm stopped: {reason}");
                *stop = Some(StopInfo {
                    reason,
                    at: epoch(),
                });
            }
        }
    }
    #[allow(dead_code)]
    pub fn stop_all(&self) {
        self.stop_all_with("operator stop");
    }
    pub fn stop_all_with(&self, reason: impl Into<String>) {
        let reason = reason.into();
        self.stop_drive_with(reason.clone());
        self.stop_arm_with(reason);
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Device<T: Serialize> {
    pub enabled: bool,
    pub connected: bool,
    pub simulated: bool,
    pub updated_at: Option<f64>,
    pub error: Option<String>,
    pub data: T,
}

impl<T: Serialize + Default> Default for Device<T> {
    fn default() -> Self {
        Self {
            enabled: false,
            connected: false,
            simulated: false,
            updated_at: None,
            error: None,
            data: T::default(),
        }
    }
}

impl<T: Serialize> Device<T> {
    pub fn update(&mut self, data: T) {
        self.data = data;
        self.connected = true;
        self.updated_at = Some(epoch());
        self.error = None;
    }
    pub fn fail(&mut self, error: impl ToString) {
        self.connected = false;
        self.error = Some(error.to_string());
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DriveData {
    pub armed: bool,
    pub power: [[i16; 2]; 2],
    pub controllers: [roboteq::Telemetry; 2],
    pub yaw_control: roboteq::YawStatus,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ArmData {
    pub armed: bool,
    pub joints: Vec<f32>,
    pub pose: Option<[f32; 6]>,
    pub gripper: Option<f32>,
    pub arm_state: Option<i32>,
    pub preview: Option<crate::arm::Preview>,
    pub rear_boundary: Option<crate::arm::RearBoundary>,
    pub rear_boundary_error: Option<String>,
    pub telemetry_error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RtkData {
    pub fix: Option<rtk::Fix>,
    pub bytes_read: u64,
    pub parse_errors: u64,
    pub ntrip_connected: bool,
    pub correction_bytes: u64,
    pub ntrip_error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Status {
    pub drive: Device<DriveData>,
    pub kinova: Device<ArmData>,
    pub rtk: Device<RtkData>,
    pub zed: Device<zed::Snapshot>,
    pub oak: Device<oakd::Snapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_stop_reason_is_recorded_only_while_armed() {
        let (state, _) = AppState::new(Default::default(), String::new(), true);
        state.stop_drive_with("operator stop");
        assert!(state.drive_stop_info.lock().unwrap().is_none());

        state.drive.lock().unwrap().enable(Instant::now());
        state.stop_drive_with("operator stop");
        assert_eq!(
            state
                .drive_stop_info
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .reason,
            "operator stop"
        );

        state.drive.lock().unwrap().enable(Instant::now());
        state.record_drive_stop("first");
        state.stop_drive_preserving_reason("second");
        assert_eq!(
            state
                .drive_stop_info
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .reason,
            "first"
        );

        state.clear_drive_stop();
        assert!(state.drive_stop_info.lock().unwrap().is_none());
    }

    #[test]
    fn arm_stop_reason_is_recorded_only_while_armed() {
        let (state, _) = AppState::new(Default::default(), String::new(), true);
        state.stop_arm_with("operator stop");
        assert!(state.arm_stop_info.lock().unwrap().is_none());

        state.arm_enabled.store(true, Ordering::SeqCst);
        state.stop_arm_with("arm heartbeat expired (500 ms)");
        assert_eq!(
            state.arm_stop_info.lock().unwrap().as_ref().unwrap().reason,
            "arm heartbeat expired (500 ms)"
        );

        state.clear_arm_stop();
        assert!(state.arm_stop_info.lock().unwrap().is_none());
    }
}
