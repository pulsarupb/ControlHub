use crate::{
    api::{OpError, arm_jog_cmd, drive_control, drive_enable, global_stop},
    arm,
    state::{AppState, DevicesSummary, DriveSummary, epoch},
};
use axum::{
    Json,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const DRIVE_PUSH_INTERVAL: Duration = Duration::from_millis(100);
const DEVICES_PUSH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum ClientFrame {
    #[serde(rename = "c")]
    Control {
        id: Option<u64>,
        th: f32,
        st: f32,
    },
    Enable {
        id: Option<u64>,
    },
    Stop {
        id: Option<u64>,
        r: Option<String>,
    },
    Jog {
        id: Option<u64>,
        c: Option<[f32; 6]>,
        j: Option<Vec<f32>>,
    },
    Take {
        id: Option<u64>,
    },
}

#[derive(Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum ServerFrame<'a> {
    Ack {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        ok: bool,
    },
    Err {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        code: &'a str,
        msg: &'a str,
    },
    Drive {
        #[serde(flatten)]
        summary: DriveSummary,
        ts: f64,
    },
    Devices {
        #[serde(flatten)]
        summary: Box<DevicesSummary>,
    },
    Stopped {
        dev: &'a [&'static str],
        reason: &'a str,
    },
}

pub async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !origin_allowed(headers.get(header::ORIGIN).and_then(|v| v.to_str().ok())) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "origin not allowed"})),
        )
            .into_response();
    }
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

fn origin_allowed(origin: Option<&str>) -> bool {
    let Some(raw) = origin else {
        // Non-browser native clients (CLI, integration tests, native Tauri WS) omit Origin.
        return true;
    };
    let origin = raw.trim().trim_end_matches('/');
    if matches!(
        origin,
        "tauri://localhost" | "http://tauri.localhost" | "https://tauri.localhost"
    ) {
        return true;
    }

    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    if scheme != "http" && scheme != "https" {
        return false;
    }
    let host = rest.split(':').next().unwrap_or("");
    if matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]") {
        return true;
    }

    if let Ok(extra) = std::env::var("ROVER_ALLOWED_ORIGINS") {
        return extra
            .split(',')
            .map(|item| item.trim().trim_end_matches('/'))
            .any(|allowed| !allowed.is_empty() && allowed == origin);
    }

    false
}

fn try_acquire_owner(state: &AppState, conn_id: u64) -> bool {
    let mut owner = state.ws_owner.lock().unwrap();
    if owner.is_none() {
        *owner = Some(conn_id);
        true
    } else {
        false
    }
}

fn is_owner(state: &AppState, conn_id: u64) -> bool {
    *state.ws_owner.lock().unwrap() == Some(conn_id)
}

fn take_owner(state: &AppState, conn_id: u64) {
    *state.ws_owner.lock().unwrap() = Some(conn_id);
}

fn release_owner(state: &AppState, conn_id: u64) -> bool {
    let mut owner = state.ws_owner.lock().unwrap();
    if *owner == Some(conn_id) {
        *owner = None;
        true
    } else {
        false
    }
}

async fn send_frame<T: Serialize>(socket: &mut WebSocket, frame: &T) -> Result<(), ()> {
    let text = serde_json::to_string(frame).map_err(|_| ())?;
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| ())
}

async fn send_ack(socket: &mut WebSocket, id: Option<u64>) -> Result<(), ()> {
    send_frame(socket, &ServerFrame::Ack { id, ok: true }).await
}

async fn send_err(
    socket: &mut WebSocket,
    id: Option<u64>,
    code: &str,
    msg: &str,
) -> Result<(), ()> {
    send_frame(socket, &ServerFrame::Err { id, code, msg }).await
}

async fn handle_socket(state: AppState, mut socket: WebSocket) {
    let conn_id = state.ws_next_id.fetch_add(1, Ordering::SeqCst);
    let mut owner = try_acquire_owner(&state, conn_id);
    let mut stop_rx = state.stop_events.subscribe();

    if !owner {
        let _ = send_err(
            &mut socket,
            None,
            "busy",
            "another operator owns the control link",
        )
        .await;
    }

    // Push an initial snapshot immediately so the client does not wait up to 500 ms.
    if owner {
        let drive = ServerFrame::Drive {
            summary: state.drive_summary(),
            ts: epoch(),
        };
        if send_frame(&mut socket, &drive).await.is_err() {
            cleanup_connection(&state, conn_id);
            return;
        }
    }
    let devices = ServerFrame::Devices {
        summary: Box::new(state.devices_summary()),
    };
    if send_frame(&mut socket, &devices).await.is_err() {
        cleanup_connection(&state, conn_id);
        return;
    }

    let mut drive_tick = tokio::time::interval(DRIVE_PUSH_INTERVAL);
    drive_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    drive_tick.tick().await;

    let mut devices_tick = tokio::time::interval(DEVICES_PUSH_INTERVAL);
    devices_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    devices_tick.tick().await;

    loop {
        tokio::select! {
            biased;
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if process_text_frame(&state, &mut socket, conn_id, &mut owner, &text).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Binary(_))) => {
                        if send_err(&mut socket, None, "bad_frame", "binary frames are not supported").await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                }
            }
            event = stop_rx.recv() => {
                match event {
                    Ok(ev) => {
                        let frame = ServerFrame::Stopped {
                            dev: &ev.devices,
                            reason: &ev.reason,
                        };
                        if send_frame(&mut socket, &frame).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = drive_tick.tick() => {
                owner = is_owner(&state, conn_id);
                if owner {
                    let frame = ServerFrame::Drive {
                        summary: state.drive_summary(),
                        ts: epoch(),
                    };
                    if send_frame(&mut socket, &frame).await.is_err() {
                        break;
                    }
                }
            }
            _ = devices_tick.tick() => {
                let frame = ServerFrame::Devices {
                    summary: Box::new(state.devices_summary()),
                };
                if send_frame(&mut socket, &frame).await.is_err() {
                    break;
                }
            }
        }
    }

    cleanup_connection(&state, conn_id);
}

fn cleanup_connection(state: &AppState, conn_id: u64) {
    if release_owner(state, conn_id) {
        state.stop_drive_with("control link closed");
    }
}

async fn process_text_frame(
    state: &AppState,
    socket: &mut WebSocket,
    conn_id: u64,
    owner: &mut bool,
    text: &str,
) -> Result<(), ()> {
    let frame: ClientFrame = match serde_json::from_str(text) {
        Ok(frame) => frame,
        Err(err) => {
            return send_err(socket, None, "bad_frame", &err.to_string()).await;
        }
    };

    // Re-check ownership in case another client sent `take`.
    *owner = is_owner(state, conn_id);

    match frame {
        ClientFrame::Take { id } => {
            take_owner(state, conn_id);
            *owner = true;
            send_ack(socket, id).await?;
            let drive = ServerFrame::Drive {
                summary: state.drive_summary(),
                ts: epoch(),
            };
            send_frame(socket, &drive).await
        }
        ClientFrame::Stop { id, r } => {
            if !*owner {
                return send_err(socket, id, "busy", "read-only connection; send take first").await;
            }
            global_stop(state, r);
            send_ack(socket, id).await
        }
        ClientFrame::Enable { id } => {
            if !*owner {
                return send_err(socket, id, "busy", "read-only connection; send take first").await;
            }
            match drive_enable(state) {
                Ok(()) => send_ack(socket, id).await,
                Err(err) => send_op_err(socket, id, err).await,
            }
        }
        ClientFrame::Control { id, th, st } => {
            if !*owner {
                return send_err(socket, id, "busy", "read-only connection; send take first").await;
            }
            match drive_control(state, th, st) {
                Ok(()) => send_ack(socket, id).await,
                Err(err) => send_op_err(socket, id, err).await,
            }
        }
        ClientFrame::Jog { id, c, j } => {
            if !*owner {
                return send_err(socket, id, "busy", "read-only connection; send take first").await;
            }
            let jog = arm::Jog {
                cartesian: c,
                joints: j,
            };
            // An all-zero jog frame acts as a heartbeat-only keepalive when not moving.
            let zero_cartesian = jog.cartesian.is_some_and(|v| v.iter().all(|&x| x == 0.0));
            let zero_joints = jog
                .joints
                .as_ref()
                .is_some_and(|v| !v.is_empty() && v.iter().all(|&x| x == 0.0));
            if jog.cartesian.is_none() && jog.joints.is_none() {
                *state.arm_seen.lock().unwrap() = Instant::now();
                return send_ack(socket, id).await;
            }
            if zero_cartesian || zero_joints {
                *state.arm_seen.lock().unwrap() = Instant::now();
                *state.arm_jog.lock().unwrap() = None;
                return send_ack(socket, id).await;
            }
            match arm_jog_cmd(state, jog) {
                Ok(()) => send_ack(socket, id).await,
                Err(err) => send_op_err(socket, id, err).await,
            }
        }
    }
}

async fn send_op_err(socket: &mut WebSocket, id: Option<u64>, err: OpError) -> Result<(), ()> {
    send_err(socket, id, err.code(), err.message()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{api::router, config::Config, motor};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::Value;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{self, client::IntoClientRequest},
    };

    type WsStream = tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >;

    async fn spawn_server(simulate: bool) -> (AppState, String) {
        let mut config = Config::default();
        config.drive.enabled = true;
        config.drive.ports = ["/dev/null/a".into(), "/dev/null/b".into()];
        let (state, _) = AppState::new(config, String::new(), simulate);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = router(state.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (state, format!("ws://{addr}/api/ws"))
    }

    fn online_drive(state: &AppState) {
        let mut s = state.status.lock().unwrap();
        s.drive.enabled = true;
        s.drive.connected = true;
    }

    async fn connect_ws(url: &str, origin: Option<&str>) -> WsStream {
        let mut req = url.into_client_request().unwrap();
        if let Some(origin) = origin {
            req.headers_mut().insert(
                "Origin",
                tungstenite::http::HeaderValue::from_str(origin).unwrap(),
            );
        }
        let (ws, _) = connect_async(req).await.unwrap();
        ws
    }

    async fn recv_json(ws: &mut WsStream) -> Value {
        loop {
            let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
                .await
                .expect("timed out waiting for ws frame")
                .expect("ws stream ended")
                .expect("ws error");
            if let tungstenite::Message::Text(text) = msg {
                return serde_json::from_str(&text).unwrap();
            }
        }
    }

    async fn recv_until(ws: &mut WsStream, mut pred: impl FnMut(&Value) -> bool) -> Value {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let v = recv_json(ws).await;
                if pred(&v) {
                    return v;
                }
            }
        })
        .await
        .expect("timed out waiting for matching ws frame")
    }

    #[tokio::test]
    async fn control_before_enable_is_rejected_then_enable_and_control_renew_lease() {
        let (state, url) = spawn_server(true).await;
        online_drive(&state);
        let mut ws = connect_ws(&url, Some("http://localhost:5173")).await;

        ws.send(tungstenite::Message::Text(
            json!({"t":"c","id":1,"th":0.3,"st":0.0}).to_string().into(),
        ))
        .await
        .unwrap();
        let err = recv_until(&mut ws, |v| v["t"] == "err" && v["id"] == 1).await;
        assert_eq!(err["code"], "bad_request");

        ws.send(tungstenite::Message::Text(
            json!({"t":"enable","id":2}).to_string().into(),
        ))
        .await
        .unwrap();
        let ack = recv_until(&mut ws, |v| v["t"] == "ack" && v["id"] == 2).await;
        assert_eq!(ack["ok"], true);
        assert!(state.drive.lock().unwrap().is_armed());

        // Keep sending at ~20 Hz past the 500 ms watchdog; lease stays armed.
        for seq in 3..18 {
            tokio::time::sleep(Duration::from_millis(50)).await;
            ws.send(tungstenite::Message::Text(
                json!({"t":"c","id":seq,"th":0.4,"st":0.0})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            let ack = recv_until(&mut ws, |v| v["t"] == "ack" && v["id"] == seq).await;
            assert_eq!(ack["ok"], true);
            assert!(state.drive.lock().unwrap().armed(Instant::now()));
        }

        // Stop sending and confirm lease expires cleanly.
        tokio::time::sleep(Duration::from_millis(550)).await;
        assert!(!state.drive.lock().unwrap().armed(Instant::now()));
    }

    #[tokio::test]
    async fn closing_socket_stops_drive_immediately_with_reason() {
        let (state, url) = spawn_server(true).await;
        online_drive(&state);
        let mut ws = connect_ws(&url, Some("tauri://localhost")).await;

        ws.send(tungstenite::Message::Text(
            json!({"t":"enable","id":1}).to_string().into(),
        ))
        .await
        .unwrap();
        recv_until(&mut ws, |v| v["t"] == "ack" && v["id"] == 1).await;
        assert!(state.drive.lock().unwrap().is_armed());

        ws.close(None).await.unwrap();
        drop(ws);

        tokio::time::timeout(Duration::from_millis(250), async {
            loop {
                if !state.drive.lock().unwrap().is_armed() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("drive did not stop promptly on socket close");

        let stop = state.drive_stop_info.lock().unwrap().clone().unwrap();
        assert_eq!(stop.reason, "control link closed");
    }

    #[tokio::test]
    async fn second_connection_is_read_only_until_take() {
        let (state, url) = spawn_server(true).await;
        online_drive(&state);

        let mut owner = connect_ws(&url, None).await;
        let _initial_drive = recv_until(&mut owner, |v| v["t"] == "drive").await;

        let mut second = connect_ws(&url, None).await;
        let busy = recv_until(&mut second, |v| v["t"] == "err" && v["code"] == "busy").await;
        assert_eq!(busy["code"], "busy");
        let _dev = recv_until(&mut second, |v| v["t"] == "devices").await;

        // Control/enable from second connection is rejected while read-only.
        second
            .send(tungstenite::Message::Text(
                json!({"t":"enable","id":10}).to_string().into(),
            ))
            .await
            .unwrap();
        let err = recv_until(&mut second, |v| v["t"] == "err" && v["id"] == 10).await;
        assert_eq!(err["code"], "busy");
        assert!(!state.drive.lock().unwrap().is_armed());

        // Second connection takes ownership, then enable succeeds.
        second
            .send(tungstenite::Message::Text(
                json!({"t":"take","id":11}).to_string().into(),
            ))
            .await
            .unwrap();
        let ack = recv_until(&mut second, |v| v["t"] == "ack" && v["id"] == 11).await;
        assert_eq!(ack["ok"], true);

        second
            .send(tungstenite::Message::Text(
                json!({"t":"enable","id":12}).to_string().into(),
            ))
            .await
            .unwrap();
        let ack = recv_until(&mut second, |v| v["t"] == "ack" && v["id"] == 12).await;
        assert_eq!(ack["ok"], true);
        assert!(state.drive.lock().unwrap().is_armed());

        // Closing the previous owner must not stop the new owner's drive.
        owner.close(None).await.unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(state.drive.lock().unwrap().is_armed());
    }

    #[tokio::test]
    async fn drive_pushes_at_ten_hz_and_devices_omit_zed_map_arrays() {
        let (state, url) = spawn_server(true).await;
        {
            let mut s = state.status.lock().unwrap();
            s.zed.data.points = vec![[1.0, 2.0, 3.0]; 100];
            s.zed.data.colors = vec![[255, 0, 0]; 100];
            s.zed.data.path = vec![zed::Pose::default(); 20];
        }

        let mut ws = connect_ws(&url, None).await;
        let devices = recv_until(&mut ws, |v| v["t"] == "devices").await;
        assert!(devices["zed"]["data"].get("points").is_none());
        assert!(devices["zed"]["data"].get("colors").is_none());
        assert!(devices["zed"]["data"].get("path").is_none());

        let mut drive_count = 0usize;
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(550) {
            if let Ok(Some(Ok(tungstenite::Message::Text(text)))) =
                tokio::time::timeout(Duration::from_millis(150), ws.next()).await
            {
                let v: Value = serde_json::from_str(&text).unwrap();
                if v["t"] == "drive" {
                    drive_count += 1;
                    assert!(v.get("armed").is_some());
                    assert!(v.get("power").is_some());
                    assert!(v.get("faults").is_some());
                }
            }
        }
        assert!(
            drive_count >= 4,
            "expected ~10 Hz drive frames, got {drive_count}"
        );
    }

    #[tokio::test]
    async fn rejected_origin_does_not_upgrade() {
        let (_, url) = spawn_server(true).await;
        let mut req = url.into_client_request().unwrap();
        req.headers_mut().insert(
            "Origin",
            tungstenite::http::HeaderValue::from_static("https://evil.example"),
        );
        let err = connect_async(req).await.unwrap_err();
        match err {
            tungstenite::Error::Http(resp) => assert_eq!(resp.status(), StatusCode::FORBIDDEN),
            other => panic!("expected HTTP 403 rejection, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn simulation_mode_drives_nonzero_power_and_emits_timeout_stop_event() {
        let (state, url) = spawn_server(true).await;
        let motor_state = state.clone();
        std::thread::spawn(move || motor::run(motor_state));

        tokio::time::timeout(Duration::from_secs(2), async {
            while !state.status.lock().unwrap().drive.connected {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();

        let mut ws = connect_ws(&url, Some("http://tauri.localhost")).await;
        ws.send(tungstenite::Message::Text(
            json!({"t":"enable","id":1}).to_string().into(),
        ))
        .await
        .unwrap();
        recv_until(&mut ws, |v| v["t"] == "ack" && v["id"] == 1).await;

        for seq in 2..10 {
            ws.send(tungstenite::Message::Text(
                json!({"t":"c","id":seq,"th":0.6,"st":0.0})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
            recv_until(&mut ws, |v| v["t"] == "ack" && v["id"] == seq).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let drive_frame = recv_until(&mut ws, |v| {
            v["t"] == "drive" && v["armed"] == true && v["power"][0][0].as_i64().unwrap_or(0) != 0
        })
        .await;
        assert_eq!(drive_frame["armed"], true);

        let stopped = recv_until(&mut ws, |v| v["t"] == "stopped").await;
        assert_eq!(stopped["dev"], json!(["drive"]));
        assert!(stopped["reason"].as_str().unwrap().contains("timeout"));
    }
}
