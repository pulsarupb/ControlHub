use crate::follower::{FollowerStatus, FollowerTarget};
use crate::localizer::Pose2d;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) rover: Arc<Mutex<RoverState>>,
    pub(crate) simulate: bool,
    pub(crate) urdf_json: Arc<String>,
}

impl AppState {
    pub(crate) fn new(simulate: bool, urdf_json: String) -> Self {
        Self {
            rover: Arc::new(Mutex::new(RoverState::new())),
            simulate,
            urdf_json: Arc::new(urdf_json),
        }
    }
}

#[derive(Debug)]
pub(crate) struct RoverState {
    pub(crate) throttle: f32,
    pub(crate) steering: f32,
    pub(crate) emergency_stop: bool,
    pub(crate) watchdog_stopped: bool,
    pub(crate) reset_requested: bool,
    pub(crate) last_ui_seen: Instant,
    pub(crate) pose: Pose2d,
    pub(crate) path: Vec<Pose2d>,
    pub(crate) motors: [MotorTelemetry; 4],
    pub(crate) last_error: Option<String>,
    pub(crate) follower_target: Option<FollowerTarget>,
    pub(crate) follower_status: FollowerStatus,
}

impl RoverState {
    pub(crate) fn new() -> Self {
        Self {
            throttle: 0.0,
            steering: 0.0,
            emergency_stop: true,
            watchdog_stopped: true,
            reset_requested: false,
            last_ui_seen: Instant::now(),
            pose: Pose2d::default(),
            path: vec![Pose2d::default()],
            motors: [
                MotorTelemetry::new(1),
                MotorTelemetry::new(2),
                MotorTelemetry::new(3),
                MotorTelemetry::new(4),
            ],
            last_error: None,
            follower_target: None,
            follower_status: FollowerStatus::default(),
        }
    }

    pub(crate) fn status(&self) -> StatusResponse {
        StatusResponse {
            throttle: self.throttle,
            steering: self.steering,
            emergency_stop: self.emergency_stop,
            watchdog_stopped: self.watchdog_stopped,
            pose: self.pose.into(),
            path: self.path.iter().copied().map(Into::into).collect(),
            motors: self.motors,
            last_error: self.last_error.clone(),
            follower: self.follower_status,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct MotorTelemetry {
    pub(crate) id: u8,
    pub(crate) position: f32,
    pub(crate) velocity: f32,
    pub(crate) fault: i8,
}

impl MotorTelemetry {
    pub(crate) fn new(id: u8) -> Self {
        Self {
            id,
            position: 0.0,
            velocity: 0.0,
            fault: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ControlRequest {
    pub(crate) throttle: f32,
    pub(crate) steering: f32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FollowTargetRequest {
    pub(crate) x_m: f32,
    pub(crate) y_m: f32,
}

#[derive(Debug, Serialize)]
pub(crate) struct StatusResponse {
    throttle: f32,
    steering: f32,
    emergency_stop: bool,
    watchdog_stopped: bool,
    pose: Pose2dResponse,
    path: Vec<Pose2dResponse>,
    motors: [MotorTelemetry; 4],
    last_error: Option<String>,
    follower: FollowerStatus,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct Pose2dResponse {
    x_m: f32,
    y_m: f32,
    heading_rad: f32,
}

impl From<Pose2d> for Pose2dResponse {
    fn from(pose: Pose2d) -> Self {
        Self {
            x_m: pose.x_m,
            y_m: pose.y_m,
            heading_rad: pose.heading_rad,
        }
    }
}
