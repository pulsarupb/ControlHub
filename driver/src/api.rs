use crate::assets::static_handler;
use crate::localizer::Pose2d;
use crate::state::{AppState, ControlRequest, StatusResponse};
use axum::{Json, Router, extract::State, routing::get, routing::post};
use std::time::Instant;

pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/status", get(api_status))
        .route("/api/control", post(api_control))
        .route("/api/heartbeat", post(api_heartbeat))
        .route("/api/stop", post(api_stop))
        .route("/api/reset", post(api_reset))
        .fallback(static_handler)
        .with_state(state)
}

async fn api_status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(state.rover.lock().expect("rover state poisoned").status())
}

async fn api_control(
    State(state): State<AppState>,
    Json(payload): Json<ControlRequest>,
) -> Json<StatusResponse> {
    let mut rover = state.rover.lock().expect("rover state poisoned");
    rover.throttle = payload.throttle.clamp(-1.0, 1.0);
    rover.steering = payload.steering.clamp(-1.0, 1.0);
    rover.last_ui_seen = Instant::now();
    rover.watchdog_stopped = false;
    Json(rover.status())
}

async fn api_heartbeat(State(state): State<AppState>) -> Json<StatusResponse> {
    let mut rover = state.rover.lock().expect("rover state poisoned");
    rover.last_ui_seen = Instant::now();
    rover.watchdog_stopped = false;
    Json(rover.status())
}

async fn api_stop(State(state): State<AppState>) -> Json<StatusResponse> {
    let mut rover = state.rover.lock().expect("rover state poisoned");
    rover.throttle = 0.0;
    rover.steering = 0.0;
    rover.emergency_stop = true;
    Json(rover.status())
}

async fn api_reset(State(state): State<AppState>) -> Json<StatusResponse> {
    let mut rover = state.rover.lock().expect("rover state poisoned");
    rover.throttle = 0.0;
    rover.steering = 0.0;
    rover.emergency_stop = false;
    rover.watchdog_stopped = false;
    rover.reset_requested = true;
    rover.last_ui_seen = Instant::now();
    rover.pose = Pose2d::default();
    rover.path.clear();
    rover.path.push(Pose2d::default());
    rover.last_error = None;
    Json(rover.status())
}
