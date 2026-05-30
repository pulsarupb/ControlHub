use crate::follower::{
    FollowerStatus, clamp_relative_target, relative_to_world_target,
};
use crate::localizer::Pose2d;
use crate::state::{AppState, ControlRequest, FollowTargetRequest, StatusResponse};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get, routing::post};
use serde::Serialize;
use std::time::Instant;
use tower_http::cors::CorsLayer;

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    ok: bool,
    version: &'static str,
}

pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(api_health))
        .route("/api/status", get(api_status))
        .route("/api/control", post(api_control))
        .route("/api/follow-target", post(api_follow_target))
        .route("/api/follow-target/cancel", post(api_cancel_follow_target))
        .route("/api/heartbeat", post(api_heartbeat))
        .route("/api/stop", post(api_stop))
        .route("/api/reset", post(api_reset))
        .with_state(state)
        .layer(CorsLayer::permissive())
}

async fn api_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "pulsar-rover",
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn api_status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(state.rover.lock().expect("rover state poisoned").status())
}

async fn api_control(
    State(state): State<AppState>,
    Json(payload): Json<ControlRequest>,
) -> impl IntoResponse {
    let mut rover = state.rover.lock().expect("rover state poisoned");
    if rover.follower_target.is_some() {
        rover.last_ui_seen = Instant::now();
        return (StatusCode::CONFLICT, Json(rover.status()));
    }

    rover.throttle = payload.throttle.clamp(-1.0, 1.0);
    rover.steering = payload.steering.clamp(-1.0, 1.0);
    rover.last_ui_seen = Instant::now();
    rover.watchdog_stopped = false;
    (StatusCode::OK, Json(rover.status()))
}

async fn api_follow_target(
    State(state): State<AppState>,
    Json(payload): Json<FollowTargetRequest>,
) -> impl IntoResponse {
    let mut rover = state.rover.lock().expect("rover state poisoned");
    if rover.emergency_stop {
        return (StatusCode::CONFLICT, Json(rover.status()));
    }

    let (relative_x_m, relative_y_m) = clamp_relative_target(payload.x_m, payload.y_m);
    let target = relative_to_world_target(rover.pose, relative_x_m, relative_y_m);

    rover.throttle = 0.0;
    rover.steering = 0.0;
    rover.follower_target = Some(target);
    rover.follower_status = FollowerStatus {
        active: true,
        target: Some(target),
        distance_m: relative_x_m.hypot(relative_y_m),
        heading_error_rad: 0.0,
        arrived: false,
    };
    rover.last_ui_seen = Instant::now();
    rover.watchdog_stopped = false;

    (StatusCode::OK, Json(rover.status()))
}

async fn api_cancel_follow_target(State(state): State<AppState>) -> Json<StatusResponse> {
    let mut rover = state.rover.lock().expect("rover state poisoned");
    rover.throttle = 0.0;
    rover.steering = 0.0;
    rover.follower_target = None;
    rover.follower_status = FollowerStatus::default();
    rover.last_ui_seen = Instant::now();
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
    rover.follower_target = None;
    rover.follower_status = FollowerStatus::default();
    rover.emergency_stop = true;
    Json(rover.status())
}

async fn api_reset(State(state): State<AppState>) -> Json<StatusResponse> {
    let mut rover = state.rover.lock().expect("rover state poisoned");
    rover.throttle = 0.0;
    rover.steering = 0.0;
    rover.emergency_stop = false;
    rover.watchdog_stopped = false;
    rover.follower_target = None;
    rover.follower_status = FollowerStatus::default();
    rover.reset_requested = true;
    rover.last_ui_seen = Instant::now();
    rover.pose = Pose2d::default();
    rover.path.clear();
    rover.path.push(Pose2d::default());
    rover.last_error = None;
    Json(rover.status())
}
