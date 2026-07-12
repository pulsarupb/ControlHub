use crate::localizer::{Chassis, ChassisConfig, Pose2d};
use crate::state::{AppState, MotorTelemetry};
use moteus::query::QueryResult;
use moteus::{BlockingController, command::PositionCommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const UI_WATCHDOG_TIMEOUT: Duration = Duration::from_millis(500);
pub(crate) const MOTOR_PERIOD: Duration = Duration::from_millis(20);
const MOTEUS_WATCHDOG_TIMEOUT_S: f32 = 0.25;
const RETRY_ATTEMPTS: u32 = 5;
const RETRY_DELAY_MS: u64 = 500;

pub(crate) struct TankDriveCommand {
    pub(crate) left_velocity: f32,
    pub(crate) right_velocity: f32,
    pub(crate) should_stop: bool,
    pub(crate) reset_requested: bool,
}

pub(crate) fn compute_tank_drive(
    rover: &mut crate::state::RoverState,
    max_velocity: f32,
) -> TankDriveCommand {
    let timed_out = rover.last_ui_seen.elapsed() > UI_WATCHDOG_TIMEOUT;
    if timed_out {
        rover.throttle = 0.0;
        rover.steering = 0.0;
        rover.watchdog_stopped = true;
    }

    let should_stop = rover.emergency_stop || rover.watchdog_stopped;
    let (throttle, steering) = if should_stop {
        (0.0, 0.0)
    } else {
        (rover.throttle, rover.steering)
    };

    let steering = steering.clamp(-1.0, 1.0);
    let throttle = throttle.clamp(-1.0, 1.0);

    let left = throttle + steering;
    let right = throttle - steering;

    let left = left.clamp(-1.0, 1.0) * max_velocity;
    let right = right.clamp(-1.0, 1.0) * max_velocity;

    let reset_requested = rover.reset_requested;
    rover.reset_requested = false;

    TankDriveCommand {
        left_velocity: left,
        right_velocity: right,
        should_stop,
        reset_requested,
    }
}

pub(crate) fn run_motor_loop(
    state: AppState,
    shared_config: Arc<Mutex<ChassisConfig>>,
    restart_flag: Arc<AtomicBool>,
) {
    if let Err(err) = run_motor_loop_inner(state.clone(), shared_config, restart_flag) {
        let mut rover = state.rover.lock().expect("rover state poisoned");
        rover.emergency_stop = true;
        rover.last_error = Some(err.to_string());
        eprintln!("Motor loop stopped: {err}");
    }
}

fn run_motor_loop_inner(
    state: AppState,
    shared_config: Arc<Mutex<ChassisConfig>>,
    restart_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let initial_config = shared_config.lock().map_err(|e| e.to_string())?;
    let mut lf_ctrl = create_stopped_controller(initial_config.left_front_id).map_err(|e| {
        format!(
            "Failed to create controller for motor {}: {e}",
            initial_config.left_front_id
        )
    })?;
    let mut rf_ctrl = create_stopped_controller(initial_config.right_front_id).map_err(|e| {
        format!(
            "Failed to create controller for motor {}: {e}",
            initial_config.right_front_id
        )
    })?;
    let mut lb_ctrl = create_stopped_controller(initial_config.left_back_id).map_err(|e| {
        format!(
            "Failed to create controller for motor {}: {e}",
            initial_config.left_back_id
        )
    })?;
    let mut rb_ctrl = create_stopped_controller(initial_config.right_back_id).map_err(|e| {
        format!(
            "Failed to create controller for motor {}: {e}",
            initial_config.right_back_id
        )
    })?;

    let mut lf_id = initial_config.left_front_id;
    let mut rf_id = initial_config.right_front_id;
    let mut lb_id = initial_config.left_back_id;
    let mut rb_id = initial_config.right_back_id;
    let mut chassis = Chassis::new(*initial_config);
    drop(initial_config);
    let mut next_tick = Instant::now();

    loop {
        if restart_flag.load(Ordering::SeqCst) {
            return Ok(());
        }

        next_tick += MOTOR_PERIOD;

        // Read latest config
        let config = shared_config.lock().map_err(|e| e.to_string())?;

        // Recreate controllers if motor IDs changed
        if config.left_front_id != lf_id {
            if let Ok(ctrl) = create_stopped_controller(config.left_front_id) {
                lf_ctrl = ctrl;
                lf_id = config.left_front_id;
            }
        }
        if config.right_front_id != rf_id {
            if let Ok(ctrl) = create_stopped_controller(config.right_front_id) {
                rf_ctrl = ctrl;
                rf_id = config.right_front_id;
            }
        }
        if config.left_back_id != lb_id {
            if let Ok(ctrl) = create_stopped_controller(config.left_back_id) {
                lb_ctrl = ctrl;
                lb_id = config.left_back_id;
            }
        }
        if config.right_back_id != rb_id {
            if let Ok(ctrl) = create_stopped_controller(config.right_back_id) {
                rb_ctrl = ctrl;
                rb_id = config.right_back_id;
            }
        }

        let max_velocity = config.max_velocity;
        let dir = (
            config.left_front_direction,
            config.right_front_direction,
            config.left_back_direction,
            config.right_back_direction,
        );

        chassis.config = *config;
        drop(config);

        let cmd = {
            let mut rover = state.rover.lock().expect("rover state poisoned");
            compute_tank_drive(&mut rover, max_velocity)
        };

        if cmd.reset_requested {
            chassis.reset_pose(Pose2d::default());
        }

        let (lf_res, rf_res, lb_res, rb_res) = if cmd.should_stop {
            let lf = motor_op(&mut lf_ctrl, lf_id, |c| c.set_stop());
            let rf = motor_op(&mut rf_ctrl, rf_id, |c| c.set_stop());
            let lb = motor_op(&mut lb_ctrl, lb_id, |c| c.set_stop());
            let rb = motor_op(&mut rb_ctrl, rb_id, |c| c.set_stop());
            (lf, rf, lb, rb)
        } else {
            let lf = motor_op(&mut lf_ctrl, lf_id, |c| {
                c.set_position(velocity_command(cmd.left_velocity * dir.0))
            });
            let rf = motor_op(&mut rf_ctrl, rf_id, |c| {
                c.set_position(velocity_command(cmd.right_velocity * dir.1))
            });
            let lb = motor_op(&mut lb_ctrl, lb_id, |c| {
                c.set_position(velocity_command(cmd.left_velocity * dir.2))
            });
            let rb = motor_op(&mut rb_ctrl, rb_id, |c| {
                c.set_position(velocity_command(cmd.right_velocity * dir.3))
            });
            (lf, rf, lb, rb)
        };

        process_results(
            &state,
            &mut chassis,
            lf_id,
            rf_id,
            lb_id,
            rb_id,
            lf_res,
            rf_res,
            lb_res,
            rb_res,
        );

        let now = Instant::now();
        if next_tick > now {
            std::thread::sleep(next_tick - now);
        } else {
            next_tick = now;
        }
    }
}

fn motor_op<F>(ctrl: &mut BlockingController, id: u8, op: F) -> Result<QueryResult, String>
where
    F: Fn(&mut BlockingController) -> Result<QueryResult, moteus::Error>,
{
    let mut last_err = None;
    for attempt in 1..=RETRY_ATTEMPTS {
        match op(ctrl) {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_err = Some(e.to_string());
                if attempt < RETRY_ATTEMPTS {
                    if let Ok(new_ctrl) = BlockingController::new(id) {
                        *ctrl = new_ctrl;
                        if let Ok(result) = op(ctrl) {
                            return Ok(result);
                        }
                    }
                    std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "unknown error".to_string()))
}

fn process_results(
    state: &AppState,
    chassis: &mut Chassis,
    lf_id: u8,
    rf_id: u8,
    lb_id: u8,
    rb_id: u8,
    lf: Result<QueryResult, String>,
    rf: Result<QueryResult, String>,
    lb: Result<QueryResult, String>,
    rb: Result<QueryResult, String>,
) {
    let mut rover = state.rover.lock().expect("rover state poisoned");

    match (&lf, &rf, &lb, &rb) {
        (Ok(lf_fb), Ok(rf_fb), Ok(lb_fb), Ok(rb_fb)) => {
            let pose = chassis.update_from_four_motors(
                lf_fb.position,
                lb_fb.position,
                rf_fb.position,
                rb_fb.position,
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
                    id: lf_id,
                    position: lf_fb.position,
                    velocity: lf_fb.velocity,
                    fault: lf_fb.fault,
                },
                MotorTelemetry {
                    id: rf_id,
                    position: rf_fb.position,
                    velocity: rf_fb.velocity,
                    fault: rf_fb.fault,
                },
                MotorTelemetry {
                    id: lb_id,
                    position: lb_fb.position,
                    velocity: lb_fb.velocity,
                    fault: lb_fb.fault,
                },
                MotorTelemetry {
                    id: rb_id,
                    position: rb_fb.position,
                    velocity: rb_fb.velocity,
                    fault: rb_fb.fault,
                },
            ];
            rover.last_error = None;
        }
        _ => {
            let mut failed = Vec::new();
            if lf.is_err() {
                failed.push(lf_id.to_string());
            }
            if rf.is_err() {
                failed.push(rf_id.to_string());
            }
            if lb.is_err() {
                failed.push(lb_id.to_string());
            }
            if rb.is_err() {
                failed.push(rb_id.to_string());
            }
            rover.last_error = Some(format!("motor(s) {} failed", failed.join(", ")));
        }
    }
}

fn create_stopped_controller(id: u8) -> Result<BlockingController, moteus::Error> {
    let mut last_err = None;
    for _ in 0..RETRY_ATTEMPTS {
        match BlockingController::new(id) {
            Ok(mut ctrl) => match ctrl.set_stop() {
                Ok(_) => return Ok(ctrl),
                Err(e) => {
                    last_err = Some(e);
                    std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                }
            },
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
            }
        }
    }
    Err(last_err.unwrap())
}

fn velocity_command(velocity: f32) -> PositionCommand {
    let mut command = PositionCommand::new();
    command.position = Some(f32::NAN);
    command.velocity = Some(velocity);
    command.watchdog_timeout = Some(MOTEUS_WATCHDOG_TIMEOUT_S);
    command
}

pub(crate) fn read_motor_continuously(motor_id: u8) -> Result<(), moteus::Error> {
    let mut ctrl = BlockingController::new(motor_id)?;
    ctrl.set_stop()?;

    let mut initial_position = None;

    loop {
        match ctrl.query() {
            Ok(result) => {
                let initial_position = *initial_position.get_or_insert(result.position);
                let delta_motor_rotations = result.position - initial_position;

                println!(
                    "Motor {} - pos={:.6} vel={:.4} fault={} delta_rot={:.4}",
                    motor_id, result.position, result.velocity, result.fault, delta_motor_rotations
                );
            }
            Err(err) => {
                println!("Error querying motor {}: {}", motor_id, err);
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
