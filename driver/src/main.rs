use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use moteus::{BlockingController, command::PositionCommand};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod localizer;

use localizer::{Chassis, ChassisConfig, Pose2d};

const MAX_MOTOR_VELOCITY: f32 = 2.0;
const UI_WATCHDOG_TIMEOUT: Duration = Duration::from_millis(500);
const MOTOR_PERIOD: Duration = Duration::from_millis(20);
const MOTEUS_WATCHDOG_TIMEOUT_S: f32 = 0.25;

const CHASSIS_CONFIG: ChassisConfig = ChassisConfig {
    wheel_radius_m: 21.59 / 2.0 / 100.0,
    track_width_m: 44.0 / 100.0,
    motor_rotations_per_wheel_rotation: 1.0,
    left_front_direction: 1.0,
    right_front_direction: -1.0,
    left_back_direction: -1.0,
    right_back_direction: 1.0,
};

#[derive(RustEmbed)]
#[folder = "../web/build"]
struct WebAssets;

#[derive(Clone)]
struct AppState {
    rover: Arc<Mutex<RoverState>>,
}

#[derive(Debug)]
struct RoverState {
    throttle: f32,
    steering: f32,
    emergency_stop: bool,
    watchdog_stopped: bool,
    reset_requested: bool,
    last_ui_seen: Instant,
    pose: Pose2d,
    path: Vec<Pose2d>,
    motors: [MotorTelemetry; 4],
    last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct MotorTelemetry {
    id: u8,
    position: f32,
    velocity: f32,
    fault: i8,
}

#[derive(Debug, Deserialize)]
struct ControlRequest {
    throttle: f32,
    steering: f32,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    throttle: f32,
    steering: f32,
    emergency_stop: bool,
    watchdog_stopped: bool,
    pose: Pose2dResponse,
    path: Vec<Pose2dResponse>,
    motors: [MotorTelemetry; 4],
    last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct Pose2dResponse {
    x_m: f32,
    y_m: f32,
    heading_rad: f32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if matches!(args.get(1).map(String::as_str), Some("read" | "calibrate")) {
        let Some(motor_id) = args.get(2).and_then(|value| value.parse::<u8>().ok()) else {
            println!("Usage: cargo run -p driver -- read <motor_id>");
            return Ok(());
        };

        read_motor_continuously(motor_id)?;
        return Ok(());
    }

    let state = AppState {
        rover: Arc::new(Mutex::new(RoverState::new())),
    };

    let motor_state = state.clone();
    std::thread::spawn(move || run_motor_loop(motor_state));

    let app = Router::new()
        .route("/api/status", get(api_status))
        .route("/api/control", post(api_control))
        .route("/api/heartbeat", post(api_heartbeat))
        .route("/api/stop", post(api_stop))
        .route("/api/reset", post(api_reset))
        .fallback(static_handler)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Rover UI listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

impl RoverState {
    fn new() -> Self {
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
        }
    }

    fn status(&self) -> StatusResponse {
        StatusResponse {
            throttle: self.throttle,
            steering: self.steering,
            emergency_stop: self.emergency_stop,
            watchdog_stopped: self.watchdog_stopped,
            pose: self.pose.into(),
            path: self.path.iter().copied().map(Into::into).collect(),
            motors: self.motors,
            last_error: self.last_error.clone(),
        }
    }
}

impl MotorTelemetry {
    fn new(id: u8) -> Self {
        Self {
            id,
            position: 0.0,
            velocity: 0.0,
            fault: 0,
        }
    }
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

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    serve_asset(path).unwrap_or_else(|| {
        serve_asset("index.html").unwrap_or_else(|| {
            (StatusCode::NOT_FOUND, "embedded web app was not built").into_response()
        })
    })
}

fn serve_asset(path: &str) -> Option<Response> {
    let asset = WebAssets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(asset.data.into_owned()))
            .expect("static asset response should build"),
    )
}

fn run_motor_loop(state: AppState) {
    if let Err(err) = run_motor_loop_inner(state.clone()) {
        let mut rover = state.rover.lock().expect("rover state poisoned");
        rover.emergency_stop = true;
        rover.last_error = Some(err.to_string());
        eprintln!("Motor loop stopped: {err}");
    }
}

fn run_motor_loop_inner(state: AppState) -> Result<(), moteus::Error> {
    let mut ctrl1 = create_stopped_controller(1)?;
    let mut ctrl2 = create_stopped_controller(2)?;
    let mut ctrl3 = create_stopped_controller(3)?;
    let mut ctrl4 = create_stopped_controller(4)?;

    let mut chassis = Chassis::new(CHASSIS_CONFIG);
    let mut next_tick = Instant::now();

    loop {
        next_tick += MOTOR_PERIOD;

        let (left_velocity, right_velocity, should_stop, reset_requested) = {
            let mut rover = state.rover.lock().expect("rover state poisoned");
            let timed_out = rover.last_ui_seen.elapsed() > UI_WATCHDOG_TIMEOUT;
            if timed_out {
                rover.throttle = 0.0;
                rover.steering = 0.0;
                rover.watchdog_stopped = true;
            }

            let should_stop = rover.emergency_stop || rover.watchdog_stopped;
            let throttle = if should_stop { 0.0 } else { rover.throttle };
            let steering = if should_stop { 0.0 } else { rover.steering };
            let left = (throttle + steering).clamp(-1.0, 1.0) * MAX_MOTOR_VELOCITY;
            let right = (throttle - steering).clamp(-1.0, 1.0) * MAX_MOTOR_VELOCITY;
            let reset_requested = rover.reset_requested;
            rover.reset_requested = false;
            (left, right, should_stop, reset_requested)
        };

        if reset_requested {
            chassis.reset_pose(Pose2d::default());
        }

        let result1;
        let result2;
        let result3;
        let result4;

        if should_stop {
            ctrl1.set_stop()?;
            ctrl2.set_stop()?;
            ctrl3.set_stop()?;
            ctrl4.set_stop()?;
            result1 = ctrl1.query();
            result2 = ctrl2.query();
            result3 = ctrl3.query();
            result4 = ctrl4.query();
        } else {
            result1 = ctrl1.set_position(velocity_command(
                left_velocity * CHASSIS_CONFIG.left_front_direction,
            ));
            result2 = ctrl2.set_position(velocity_command(
                right_velocity * CHASSIS_CONFIG.right_back_direction,
            ));
            result3 = ctrl3.set_position(velocity_command(
                right_velocity * CHASSIS_CONFIG.right_front_direction,
            ));
            result4 = ctrl4.set_position(velocity_command(
                left_velocity * CHASSIS_CONFIG.left_back_direction,
            ));
        }

        let mut rover = state.rover.lock().expect("rover state poisoned");
        match (&result1, &result2, &result3, &result4) {
            (Ok(feedback1), Ok(feedback2), Ok(feedback3), Ok(feedback4)) => {
                let pose = chassis.update_from_four_motors(
                    feedback1.position,
                    feedback4.position,
                    feedback3.position,
                    feedback2.position,
                );
                rover.pose = pose;

                let should_push = rover
                    .path
                    .last()
                    .map(|last| {
                        (last.x_m - pose.x_m).hypot(last.y_m - pose.y_m) > 0.01
                            || (last.heading_rad - pose.heading_rad).abs() > 0.05
                    })
                    .unwrap_or(true);
                if should_push {
                    rover.path.push(pose);
                    if rover.path.len() > 2_000 {
                        rover.path.remove(0);
                    }
                }

                rover.motors = [
                    MotorTelemetry {
                        id: 1,
                        position: feedback1.position,
                        velocity: feedback1.velocity,
                        fault: feedback1.fault,
                    },
                    MotorTelemetry {
                        id: 2,
                        position: feedback2.position,
                        velocity: feedback2.velocity,
                        fault: feedback2.fault,
                    },
                    MotorTelemetry {
                        id: 3,
                        position: feedback3.position,
                        velocity: feedback3.velocity,
                        fault: feedback3.fault,
                    },
                    MotorTelemetry {
                        id: 4,
                        position: feedback4.position,
                        velocity: feedback4.velocity,
                        fault: feedback4.fault,
                    },
                ];
                rover.last_error = None;
            }
            _ => {
                rover.last_error = Some("failed to read one or more motor controllers".to_string());
            }
        }
        drop(rover);

        let now = Instant::now();
        if next_tick > now {
            std::thread::sleep(next_tick - now);
        } else {
            next_tick = now;
        }
    }
}

fn create_stopped_controller(id: u8) -> Result<BlockingController, moteus::Error> {
    let mut ctrl = BlockingController::new(id)?;
    ctrl.set_stop()?;
    Ok(ctrl)
}

fn velocity_command(velocity: f32) -> PositionCommand {
    let mut command = PositionCommand::new();
    command.position = Some(f32::NAN);
    command.velocity = Some(velocity);
    command.watchdog_timeout = Some(MOTEUS_WATCHDOG_TIMEOUT_S);
    command
}

fn read_motor_continuously(motor_id: u8) -> Result<(), moteus::Error> {
    let mut ctrl = BlockingController::new(motor_id)?;
    ctrl.set_stop()?;

    let mut initial_position = None;

    loop {
        match ctrl.query() {
            Ok(result) => {
                let initial_position = *initial_position.get_or_insert(result.position);
                let delta_motor_rotations = result.position - initial_position;
                let delta_wheel_rotations =
                    delta_motor_rotations / CHASSIS_CONFIG.motor_rotations_per_wheel_rotation;
                let delta_m = CHASSIS_CONFIG.motor_rotations_to_meters(delta_motor_rotations);

                println!(
                    "Motor {} - mode={:?} pos={:.6} rot delta={:.6} motor_rot wheel_delta={:.6} wheel_rot distance={:.4}m vel={:.4} fault={}",
                    motor_id,
                    result.mode,
                    result.position,
                    delta_motor_rotations,
                    delta_wheel_rotations,
                    delta_m,
                    result.velocity,
                    result.fault
                );
            }
            Err(err) => {
                println!("Error querying motor {}: {}", motor_id, err);
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
