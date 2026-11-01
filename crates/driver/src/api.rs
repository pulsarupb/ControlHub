use crate::{
    arm, config,
    state::{AppState, epoch},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tower_http::cors::CorsLayer;

type Error = (StatusCode, Json<Value>);
fn bad(error: impl ToString) -> Error {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": error.to_string()})),
    )
}
fn unavailable(error: impl ToString) -> Error {
    (
        StatusCode::CONFLICT,
        Json(json!({"error": error.to_string()})),
    )
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/status", get(status))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/control", post(control))
        .route("/api/drive/enable", post(enable_drive))
        .route("/api/drive/stop", post(stop_drive))
        .route("/api/heartbeat", post(heartbeat))
        .route("/api/stop", post(stop))
        .route("/api/arm/action", post(arm_action))
        .route("/api/arm/jog", post(arm_jog))
        .route("/api/arm/stop", post(stop_arm))
        .route("/api/zed/map", get(zed_map))
        .route("/api/zed/mesh", get(zed_mesh).post(zed_generate_mesh))
        .route("/api/zed/reset", post(zed_reset))
        .route("/api/zed/frame/{kind}", get(zed_frame))
        .route("/api/oak/frame/{slot}/{kind}", get(oak_frame))
        .route("/api/d435/frame/{kind}", get(crate::d435::frame))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(
        json!({"service": "pulsar-rover", "ok": true, "version": env!("CARGO_PKG_VERSION"),
        "mode": if state.simulate { "simulated" } else { "real" }}),
    )
}

async fn oak_frame(
    State(state): State<AppState>,
    Path((slot, kind)): Path<(usize, String)>,
) -> Result<impl IntoResponse, Error> {
    let index = match kind.as_str() {
        "color" => 0,
        "depth" => 1,
        _ => return Err(bad("expected color or depth")),
    };
    if slot >= 3 {
        return Err(bad("OAK-D slot must be 0, 1 or 2"));
    }
    if !state.config.lock().unwrap().oak.enabled {
        return Err(unavailable("OAK-D disabled"));
    }
    let status = state.status.lock().unwrap();
    let camera = &status.oak.data.cameras[slot];
    if !status.oak.connected
        || !camera.connected
        || !camera
            .updated_at
            .is_some_and(|t| (0.0..3.0).contains(&(epoch() - t)))
    {
        return Err(unavailable("OAK-D camera offline or stale"));
    }
    let images = state.oak_images.lock().unwrap();
    let Some((bytes, captured)) = &images[slot][index] else {
        return Err(unavailable("No OAK-D frame available"));
    };
    if captured.elapsed() > Duration::from_secs(3) {
        return Err(unavailable("OAK-D frame stale"));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes.clone(),
    ))
}

async fn status(State(state): State<AppState>) -> Json<Value> {
    let armed = state.drive.lock().unwrap().armed(Instant::now());
    let mut status = state.status.lock().unwrap().clone();
    status.drive.data.armed = armed;
    status.kinova.data.armed = state.arm_enabled.load(Ordering::SeqCst);
    status.zed.data.path.clear();
    status.zed.data.points.clear();
    status.zed.data.colors.clear();
    Json(json!({"time": epoch(), "simulate": state.simulate, "devices": status}))
}

async fn get_config(State(state): State<AppState>) -> Json<config::Config> {
    let mut config = state.config.lock().unwrap().clone();
    // Blank means unchanged on PUT. Passwords are never returned to clients.
    config.kinova.password.clear();
    config.rtk.ntrip.password.clear();
    Json(config)
}

async fn put_config(
    State(state): State<AppState>,
    Json(mut payload): Json<config::Config>,
) -> Result<Json<Value>, Error> {
    // Serialize validation, password merging and replacement as one operation.
    let mut config = state.config.lock().unwrap();
    let old = config.clone();
    if payload.kinova.password.is_empty() {
        payload.kinova.password.clone_from(&old.kinova.password);
    }
    if payload.rtk.ntrip.password.is_empty() {
        payload
            .rtk
            .ntrip
            .password
            .clone_from(&old.rtk.ntrip.password);
    }
    payload.validate().map_err(bad)?;
    if payload.drive != old.drive || (old.drive.yaw_control.enabled && payload.zed != old.zed) {
        state.stop_drive();
        if payload.zed != old.zed {
            *state.yaw_feedback.lock().unwrap() = None;
        }
    }
    if payload.kinova != old.kinova {
        state.stop_arm();
    }
    config::save(&state.config_path, &payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))))?;
    *config = payload;
    Ok(Json(json!({"saved": true})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    throttle: f32,
    steering: f32,
}

async fn control(
    State(state): State<AppState>,
    Json(payload): Json<Control>,
) -> Result<Json<Value>, Error> {
    let available = {
        let s = state.status.lock().unwrap();
        s.drive.enabled && s.drive.connected
    };
    if !available {
        state.stop_drive();
        return Err(unavailable("drive disabled or offline"));
    }
    state
        .drive
        .lock()
        .unwrap()
        .command(payload.throttle, payload.steering, Instant::now())
        .map_err(bad)?;
    Ok(Json(json!({"ok": true})))
}

async fn enable_drive(State(state): State<AppState>) -> Result<Json<Value>, Error> {
    let mesh = state.zed_mesh.lock().unwrap();
    if mesh.generating {
        return Err(unavailable(
            "Wait for mesh generation to finish before enabling the drive",
        ));
    }
    let available = {
        let s = state.status.lock().unwrap();
        s.drive.enabled && s.drive.connected
    };
    if !available {
        return Err(unavailable("drive disabled or offline"));
    }
    let config = state.config.lock().unwrap();
    if config.drive.yaw_control.enabled {
        let feedback = state.yaw_feedback.lock().unwrap();
        if !feedback.is_some_and(|f| {
            f.received.elapsed().as_millis()
                < u128::from(config.drive.yaw_control.feedback_timeout_ms)
        }) {
            return Err(unavailable(
                "fresh measured ZED odometry required for yaw control",
            ));
        }
    }
    state.drive.lock().unwrap().enable(Instant::now());
    Ok(Json(json!({"ok": true})))
}

async fn stop_drive(State(state): State<AppState>) -> Json<Value> {
    state.stop_drive();
    Json(json!({"ok": true}))
}
async fn stop_arm(State(state): State<AppState>) -> Json<Value> {
    state.stop_arm();
    Json(json!({"ok": true}))
}
async fn stop(State(state): State<AppState>) -> Json<Value> {
    state.stop_all();
    Json(json!({"ok": true}))
}
async fn heartbeat(State(state): State<AppState>) -> Json<Value> {
    *state.arm_seen.lock().unwrap() = Instant::now();
    Json(json!({"ok": true}))
}

async fn arm_jog(
    State(state): State<AppState>,
    Json(jog): Json<arm::Jog>,
) -> Result<Json<Value>, Error> {
    jog.validate().map_err(|e| {
        state.stop_arm();
        bad(e)
    })?;
    let mut pending_jog = state.arm_jog.lock().unwrap();
    if !state.arm_enabled.load(Ordering::SeqCst) {
        return Err(unavailable("arm stopped; explicit enable required"));
    }
    *state.arm_seen.lock().unwrap() = Instant::now();
    *pending_jog = Some((jog, Instant::now()));
    Ok(Json(json!({"ok": true})))
}

async fn arm_action(
    State(state): State<AppState>,
    Json(action): Json<arm::Action>,
) -> Result<Json<Value>, Error> {
    let available = {
        let s = state.status.lock().unwrap();
        s.kinova.enabled && s.kinova.connected
    };
    if !available {
        return Err(unavailable("Kinova disabled or offline"));
    }
    let timeout = match &action {
        arm::Action::MoveCartesian { .. } => Duration::from_secs(20),
        arm::Action::InstallRearZone => Duration::from_secs(15),
        _ => Duration::from_secs(3),
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .arm_tx
        .try_send(arm::Request {
            action,
            generation: state.arm_stop.load(Ordering::SeqCst),
            reply: tx,
        })
        .map_err(unavailable)?;
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(Ok(result))) => Ok(Json(result)),
        Ok(Ok(Err(error))) => Err(bad(error)),
        _ => {
            state.stop_arm();
            Err(unavailable("arm request timed out; arm stopped"))
        }
    }
}

async fn zed_map(State(state): State<AppState>) -> Json<zed::Snapshot> {
    Json(state.status.lock().unwrap().zed.data.clone())
}
#[derive(Deserialize)]
struct MeshQuery {
    since: Option<f64>,
}
async fn zed_mesh(
    State(state): State<AppState>,
    Query(query): Query<MeshQuery>,
) -> Json<zed::MeshSnapshot> {
    let snapshot = state.zed_mesh.lock().unwrap();
    Json(zed::MeshSnapshot {
        generating: snapshot.generating,
        generated_at: snapshot.generated_at,
        error: snapshot.error.clone(),
        mesh: if query.since.is_some() && query.since == snapshot.generated_at {
            None
        } else {
            snapshot.mesh.clone()
        },
    })
}
async fn zed_generate_mesh(State(state): State<AppState>) -> Result<Json<Value>, Error> {
    {
        let status = state.status.lock().unwrap();
        if !status.zed.connected || status.zed.simulated {
            return Err(unavailable("A real ZED camera must be connected"));
        }
        if status.drive.data.armed {
            return Err(bad(
                "Disarm the drive before baking a mesh; tracking may pause",
            ));
        }
    }
    {
        let config = state.config.lock().unwrap();
        if config.zed.settings["mapping"]["enabled"] != true
            || config.zed.settings["mapping"]["map_type"] != "MESH"
        {
            return Err(bad("Enable ZED spatial mapping with map_type MESH first"));
        }
    }
    let mut snapshot = state.zed_mesh.lock().unwrap();
    if snapshot.generating {
        return Err(bad("Mesh generation is already in progress"));
    }
    snapshot.generating = true;
    snapshot.error = None;
    // Pair with enable_drive's mesh lock so a concurrent enable cannot race baking.
    state.stop_drive();
    state.zed_generate_mesh.store(true, Ordering::SeqCst);
    Ok(Json(json!({"ok": true})))
}
async fn zed_reset(State(state): State<AppState>) -> Result<Json<Value>, Error> {
    let mesh = state.zed_mesh.lock().unwrap();
    if mesh.generating {
        return Err(bad(
            "Wait for mesh generation to finish before resetting tracking",
        ));
    }
    if !state.status.lock().unwrap().zed.connected {
        return Err(unavailable("ZED offline"));
    }
    if state.config.lock().unwrap().drive.yaw_control.enabled {
        state.stop_drive();
    }
    *state.yaw_feedback.lock().unwrap() = None;
    state.zed_reset.store(true, Ordering::SeqCst);
    Ok(Json(json!({"ok": true})))
}
async fn zed_frame(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let index = match kind.as_str() {
        "color" => 0,
        "depth" => 1,
        _ => return Err(bad("unknown image kind")),
    };
    if !state.status.lock().unwrap().zed.connected {
        return Err(unavailable("ZED offline"));
    }
    let bytes = state.images.lock().unwrap()[index].clone();
    if bytes.is_empty() {
        return Err(unavailable("no ZED frame available"));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    fn setup() -> (AppState, Router) {
        let (state, _) = AppState::new(config::Config::default(), String::new(), true);
        (state.clone(), router(state))
    }
    async fn call(router: Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        let response = router
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    fn online_drive(state: &AppState) {
        let mut status = state.status.lock().unwrap();
        status.drive.enabled = true;
        status.drive.connected = true;
    }

    #[tokio::test]
    async fn oak_frames_are_independent_validated_and_never_stale() {
        let (state, router) = setup();
        assert_eq!(
            call(router.clone(), "GET", "/api/oak/frame/0/color", json!({}))
                .await
                .0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            call(router.clone(), "GET", "/api/oak/frame/3/color", json!({}))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            call(router.clone(), "GET", "/api/oak/frame/0/unknown", json!({}))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
        state.config.lock().unwrap().oak.enabled = true;
        {
            let mut status = state.status.lock().unwrap();
            status.oak.connected = true;
            status.oak.data.cameras[0].connected = true;
            status.oak.data.cameras[0].updated_at = Some(epoch());
        }
        state.oak_images.lock().unwrap()[0][0] =
            Some((vec![0xff, 0xd8, 0xff, 0xd9], Instant::now()));
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/oak/frame/0/color")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/jpeg");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            &to_bytes(response.into_body(), 100).await.unwrap()[..],
            &[0xff, 0xd8, 0xff, 0xd9]
        );
        assert_eq!(
            call(router.clone(), "GET", "/api/oak/frame/1/color", json!({}))
                .await
                .0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            call(router.clone(), "GET", "/api/oak/frame/0/depth", json!({}))
                .await
                .0,
            StatusCode::CONFLICT
        );
        state.oak_images.lock().unwrap()[0][0].as_mut().unwrap().1 =
            Instant::now() - Duration::from_secs(4);
        assert_eq!(
            call(router.clone(), "GET", "/api/oak/frame/0/color", json!({}))
                .await
                .0,
            StatusCode::CONFLICT
        );
        state.oak_images.lock().unwrap()[0][0].as_mut().unwrap().1 = Instant::now();
        state.status.lock().unwrap().oak.data.cameras[0].updated_at = Some(epoch() - 4.0);
        assert_eq!(
            call(router, "GET", "/api/oak/frame/0/color", json!({}))
                .await
                .0,
            StatusCode::CONFLICT
        );
    }

    #[tokio::test]
    async fn mesh_generation_blocks_drive_enable_and_tracking_reset() {
        let (state, router) = setup();
        online_drive(&state);
        state.zed_mesh.lock().unwrap().generating = true;
        assert_eq!(
            call(router.clone(), "POST", "/api/drive/enable", json!({}))
                .await
                .0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            call(router, "POST", "/api/zed/reset", json!({})).await.0,
            StatusCode::BAD_REQUEST
        );
        assert!(!state.drive.lock().unwrap().armed(Instant::now()));
    }

    #[tokio::test]
    async fn disabled_devices_cannot_move_or_enable() {
        let (_, router) = setup();
        for (path, body) in [
            ("/api/control", json!({"throttle": 0.1, "steering": 0})),
            ("/api/drive/enable", json!({})),
            ("/api/arm/action", json!({"action":"enable"})),
        ] {
            assert_eq!(
                call(router.clone(), "POST", path, body).await.0,
                StatusCode::CONFLICT
            );
        }
    }

    #[tokio::test]
    async fn invalid_demand_latches_stop_and_heartbeat_cannot_reenable() {
        let (state, router) = setup();
        online_drive(&state);
        assert_eq!(
            call(router.clone(), "POST", "/api/drive/enable", json!({}))
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            call(
                router.clone(),
                "POST",
                "/api/control",
                json!({"throttle": 2, "steering":0})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        call(router.clone(), "POST", "/api/heartbeat", json!({})).await;
        assert!(!state.drive.lock().unwrap().armed(Instant::now()));
        assert_eq!(
            call(
                router,
                "POST",
                "/api/control",
                json!({"throttle":0.1, "steering":0})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn expired_commands_cannot_revive_drive() {
        let (state, router) = setup();
        online_drive(&state);
        state
            .drive
            .lock()
            .unwrap()
            .enable(Instant::now() - Duration::from_secs(1));
        assert_eq!(
            call(
                router,
                "POST",
                "/api/control",
                json!({"throttle":0.1, "steering":0})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn yaw_enable_needs_fresh_feedback_and_reset_disarms() {
        let (state, router) = setup();
        online_drive(&state);
        state.config.lock().unwrap().drive.yaw_control.enabled = true;
        assert_eq!(
            call(router.clone(), "POST", "/api/drive/enable", json!({}))
                .await
                .0,
            StatusCode::CONFLICT
        );
        let now = Instant::now();
        *state.yaw_feedback.lock().unwrap() = Some(roboteq::YawFeedback {
            yaw: 0.0,
            timestamp: 1.0,
            received: now - Duration::from_secs(1),
            epoch: 0,
        });
        assert_eq!(
            call(router.clone(), "POST", "/api/drive/enable", json!({}))
                .await
                .0,
            StatusCode::CONFLICT
        );
        state
            .yaw_feedback
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .received = now;
        assert_eq!(
            call(router.clone(), "POST", "/api/drive/enable", json!({}))
                .await
                .0,
            StatusCode::OK
        );
        state.status.lock().unwrap().zed.connected = true;
        assert_eq!(
            call(router.clone(), "POST", "/api/zed/reset", json!({}))
                .await
                .0,
            StatusCode::OK
        );
        assert!(!state.drive.lock().unwrap().armed(Instant::now()));
        assert!(state.yaw_feedback.lock().unwrap().is_none());
        assert_eq!(
            call(router, "POST", "/api/drive/enable", json!({})).await.0,
            StatusCode::CONFLICT
        );
    }

    #[tokio::test]
    async fn global_stop_affects_both_motion_devices_not_sensors() {
        let (state, router) = setup();
        online_drive(&state);
        state.drive.lock().unwrap().enable(Instant::now());
        state.arm_enabled.store(true, Ordering::SeqCst);
        state.status.lock().unwrap().rtk.connected = true;
        let before = state.arm_stop.load(Ordering::SeqCst);
        call(router, "POST", "/api/stop", json!({})).await;
        assert!(!state.drive.lock().unwrap().armed(Instant::now()));
        assert!(!state.arm_enabled.load(Ordering::SeqCst));
        assert!(state.arm_stop.load(Ordering::SeqCst) > before);
        assert!(state.status.lock().unwrap().rtk.connected);
    }

    #[tokio::test]
    async fn arm_invalid_jog_stops_and_valid_jog_cannot_enable() {
        let (state, router) = setup();
        state.arm_enabled.store(true, Ordering::SeqCst);
        assert_eq!(
            call(
                router.clone(),
                "POST",
                "/api/arm/jog",
                json!({"cartesian":[0,0,0,0,0,2]})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
        assert!(!state.arm_enabled.load(Ordering::SeqCst));
        assert_eq!(
            call(
                router,
                "POST",
                "/api/arm/jog",
                json!({"cartesian":[0,0,0,0,0,0]})
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
    }

    #[tokio::test]
    async fn config_redacts_secrets_and_rejects_invalid_save() {
        let (state, router) = setup();
        state.config.lock().unwrap().kinova.password = "private-arm-secret".into();
        state.config.lock().unwrap().rtk.ntrip.password = "private-caster-secret".into();
        let (_, config) = call(router.clone(), "GET", "/api/config", json!({})).await;
        assert_eq!(config["kinova"]["password"], "");
        assert!(!config.to_string().contains("private-"));
        let mut invalid = config;
        invalid["drive"]["power_limit"] = 1001.into();
        assert_eq!(
            call(router, "PUT", "/api/config", invalid).await.0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(state.config.lock().unwrap().drive.power_limit, 100);
    }

    #[tokio::test]
    async fn changing_sensor_settings_does_not_stop_drive_and_blank_password_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = config::Config::default();
        config.kinova.password = "secret".into();
        let (state, _) = AppState::new(
            config,
            dir.path().join("config.json").to_str().unwrap().into(),
            true,
        );
        state.drive.lock().unwrap().enable(Instant::now());
        let router = router(state.clone());
        let (_, mut payload) = call(router.clone(), "GET", "/api/config", json!({})).await;
        payload["rtk"]["baud"] = 9600.into();
        assert_eq!(
            call(router, "PUT", "/api/config", payload).await.0,
            StatusCode::OK
        );
        assert!(state.drive.lock().unwrap().armed(Instant::now()));
        let saved = config::load(state.config_path.as_str()).unwrap();
        assert_eq!(saved.kinova.password, "secret");
        assert_eq!(saved.rtk.baud, 9600);
    }

    #[tokio::test]
    async fn changing_zed_mount_stops_feedback_controlled_drive() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = config::Config::default();
        config.drive.yaw_control.enabled = true;
        config.zed.enabled = true;
        config.zed.mount_configured = true;
        config.zed.settings["tracking"]["enable_area_memory"] = false.into();
        let (state, _) = AppState::new(
            config,
            dir.path().join("config.json").to_str().unwrap().into(),
            true,
        );
        state.drive.lock().unwrap().enable(Instant::now());
        let router = router(state.clone());
        let (_, mut payload) = call(router.clone(), "GET", "/api/config", json!({})).await;
        payload["zed"]["mount_rotation_deg"][2] = 90.into();
        assert_eq!(
            call(router, "PUT", "/api/config", payload).await.0,
            StatusCode::OK
        );
        assert!(!state.drive.lock().unwrap().armed(Instant::now()));
    }

    #[tokio::test]
    async fn simulated_arm_heartbeat_expiry_latches_and_ik_is_not_fabricated() {
        let mut config = config::Config::default();
        config.kinova.enabled = true;
        let (state, rx) = AppState::new(config, String::new(), true);
        let worker = tokio::spawn(arm::run(state.clone(), rx));
        tokio::time::timeout(Duration::from_secs(2), async {
            while !state.status.lock().unwrap().kinova.connected {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let router = router(state.clone());
        for action in ["retract", "home", "packaging", "zero", "rest"] {
            let (code, error) = call(
                router.clone(),
                "POST",
                "/api/arm/action",
                json!({"action": action}),
            )
            .await;
            assert_eq!(code, StatusCode::BAD_REQUEST, "{action}");
            assert!(
                error["error"]
                    .as_str()
                    .unwrap()
                    .contains("explicit enable required")
            );
        }
        assert_eq!(
            call(
                router.clone(),
                "POST",
                "/api/arm/action",
                json!({"action":"enable"})
            )
            .await
            .0,
            StatusCode::OK
        );
        assert!(state.arm_enabled.load(Ordering::SeqCst));
        for action in ["retract", "home", "packaging", "zero", "rest"] {
            call(router.clone(), "POST", "/api/heartbeat", json!({})).await;
            assert_eq!(
                call(
                    router.clone(),
                    "POST",
                    "/api/arm/action",
                    json!({"action": action})
                )
                .await
                .0,
                StatusCode::OK,
                "{action}"
            );
        }
        *state.arm_seen.lock().unwrap() = Instant::now() - Duration::from_secs(1);
        tokio::time::timeout(Duration::from_secs(2), async {
            while state.arm_enabled.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        call(router.clone(), "POST", "/api/heartbeat", json!({})).await;
        assert!(!state.arm_enabled.load(Ordering::SeqCst));
        for action in ["retract", "home", "packaging", "zero", "rest"] {
            assert_eq!(
                call(
                    router.clone(),
                    "POST",
                    "/api/arm/action",
                    json!({"action": action})
                )
                .await
                .0,
                StatusCode::BAD_REQUEST,
                "{action}"
            );
        }
        assert_eq!(
            call(
                router.clone(),
                "POST",
                "/api/arm/jog",
                json!({"cartesian":[0,0,0,0,0,0]})
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
        let (code, error) = call(
            router,
            "POST",
            "/api/arm/action",
            json!({"action":"solve","pose":[0,0,0,0,0,0]}),
        )
        .await;
        assert_eq!(code, StatusCode::BAD_REQUEST);
        assert!(
            error["error"]
                .as_str()
                .unwrap()
                .contains("no simulated solution")
        );
        assert!(state.status.lock().unwrap().kinova.data.preview.is_none());
        worker.abort();
    }
}
