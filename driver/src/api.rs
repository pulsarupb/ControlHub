use crate::config::{self, RawConfig};
use crate::localizer::Pose2d;
use crate::state::{AppState, ControlRequest, StatusResponse};
use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::CorsLayer;

#[derive(Debug, Clone, Serialize)]
struct HealthResponse {
    service: &'static str,
    ok: bool,
    version: &'static str,
    mode: &'static str,
}

pub(crate) fn router(state: AppState, restart_flag: Arc<AtomicBool>) -> Router {
    Router::new()
        .route("/api/health", get(api_health))
        .route("/api/status", get(api_status))
        .route("/api/urdf-model", get(api_urdf_model))
        .route("/api/control", post(api_control))
        .route("/api/heartbeat", post(api_heartbeat))
        .route("/api/stop", post(api_stop))
        .route("/api/reset", post(api_reset))
        .route("/api/config", get(api_get_config).put(api_put_config))
        .with_state(ApiState { state, restart_flag })
        .layer(CorsLayer::permissive())
}

#[derive(Clone)]
struct ApiState {
    state: AppState,
    restart_flag: Arc<AtomicBool>,
}

async fn api_health(State(api): State<ApiState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "pulsar-rover",
        ok: true,
        version: env!("CARGO_PKG_VERSION"),
        mode: if api.state.simulate { "simulated" } else { "real" },
    })
}

async fn api_urdf_model(State(api): State<ApiState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        (*api.state.urdf_json).clone(),
    )
}

async fn api_status(State(api): State<ApiState>) -> Json<StatusResponse> {
    Json(api.state.rover.lock().expect("rover state poisoned").status())
}

async fn api_control(
    State(api): State<ApiState>,
    Json(payload): Json<ControlRequest>,
) -> impl IntoResponse {
    let mut rover = api.state.rover.lock().expect("rover state poisoned");

    rover.throttle = payload.throttle.clamp(-1.0, 1.0);
    rover.steering = payload.steering.clamp(-1.0, 1.0);
    rover.last_ui_seen = Instant::now();
    rover.watchdog_stopped = false;
    (StatusCode::OK, Json(rover.status()))
}

async fn api_heartbeat(State(api): State<ApiState>) -> Json<StatusResponse> {
    let mut rover = api.state.rover.lock().expect("rover state poisoned");
    rover.last_ui_seen = Instant::now();
    rover.watchdog_stopped = false;
    Json(rover.status())
}

async fn api_stop(State(api): State<ApiState>) -> Json<StatusResponse> {
    let mut rover = api.state.rover.lock().expect("rover state poisoned");
    rover.throttle = 0.0;
    rover.steering = 0.0;
    rover.emergency_stop = true;
    Json(rover.status())
}

async fn api_reset(State(api): State<ApiState>) -> Json<StatusResponse> {
    let mut rover = api.state.rover.lock().expect("rover state poisoned");
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

async fn api_get_config(State(api): State<ApiState>) -> impl IntoResponse {
    let raw = {
        let config = api.state.chassis_config.lock().expect("config poisoned");
        RawConfig {
            chassis: config::RawChassis {
                wheel_radius_mm: config.wheel_radius_m * 1000.0,
                track_width_mm: config.track_width_m * 1000.0,
                motor_rotations_per_wheel_rotation: config.motor_rotations_per_wheel_rotation,
                max_velocity: config.max_velocity,
            },
            motors: config::RawMotors {
                left_front: config::RawMotor { id: config.left_front_id, direction: config.left_front_direction },
                right_front: config::RawMotor { id: config.right_front_id, direction: config.right_front_direction },
                left_back: config::RawMotor { id: config.left_back_id, direction: config.left_back_direction },
                right_back: config::RawMotor { id: config.right_back_id, direction: config.right_back_direction },
            },
        }
    };
    (StatusCode::OK, Json(raw))
}

async fn api_put_config(
    State(api): State<ApiState>,
    Json(payload): Json<RawConfig>,
) -> impl IntoResponse {
    // Validate direction values
    for (name, motor) in [("left_front", &payload.motors.left_front), ("right_front", &payload.motors.right_front), ("left_back", &payload.motors.left_back), ("right_back", &payload.motors.right_back)] {
        if motor.direction != 1.0 && motor.direction != -1.0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{name}.direction must be 1.0 or -1.0")})),
            );
        }
    }

    // Save to disk
    if let Err(e) = config::save_config(&api.state.config_path, &payload) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        );
    }

    // Update in-memory config
    {
        let mut config = api.state.chassis_config.lock().expect("config poisoned");
        *config = config::raw_to_chassis_config(&payload);
    }

    // Trigger motor loop restart
    api.restart_flag.store(true, Ordering::SeqCst);

    (StatusCode::OK, Json(serde_json::json!({"status": "saved"})))
}
