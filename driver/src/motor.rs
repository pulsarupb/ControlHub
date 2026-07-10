use crate::follower::{FollowerStatus, compute_follower_command};
use crate::localizer::{Chassis, ChassisConfig, Pose2d};
use crate::state::{AppState, MotorTelemetry};
use moteus::{BlockingController, command::PositionCommand};
use std::time::{Duration, Instant};

const MAX_MOTOR_VELOCITY: f32 = 0.25 * 8.0;
const UI_WATCHDOG_TIMEOUT: Duration = Duration::from_millis(500);
const MOTOR_PERIOD: Duration = Duration::from_millis(20);
const MOTEUS_WATCHDOG_TIMEOUT_S: f32 = 0.25;

const CHASSIS_CONFIG: ChassisConfig = ChassisConfig {
    wheel_radius_m: 21.59 / 2.0 / 100.0,
    track_width_m: 44.0 / 100.0,
    motor_rotations_per_wheel_rotation: 1.0,
    left_front_id: 1,
    left_front_direction: 1.0,
    right_front_id: 3,
    right_front_direction: -1.0,
    left_back_id: 4,
    left_back_direction: -1.0,
    right_back_id: 2,
    right_back_direction: 1.0,
};

pub(crate) fn run_motor_loop(state: AppState) {
    if let Err(err) = run_motor_loop_inner(state.clone()) {
        let mut rover = state.rover.lock().expect("rover state poisoned");
        rover.emergency_stop = true;
        rover.last_error = Some(err.to_string());
        eprintln!("Motor loop stopped: {err}");
    }
}

fn run_motor_loop_inner(state: AppState) -> Result<(), moteus::Error> {
    let mut left_front = create_stopped_controller(CHASSIS_CONFIG.left_front_id)?;
    let mut right_front = create_stopped_controller(CHASSIS_CONFIG.right_front_id)?;
    let mut left_back = create_stopped_controller(CHASSIS_CONFIG.left_back_id)?;
    let mut right_back = create_stopped_controller(CHASSIS_CONFIG.right_back_id)?;

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
                rover.follower_target = None;
                rover.follower_status = FollowerStatus::default();
                rover.watchdog_stopped = true;
            }

            let should_stop = rover.emergency_stop || rover.watchdog_stopped;
            let (throttle, steering) = if should_stop {
                (0.0, 0.0)
            } else if let Some(target) = rover.follower_target {
                let command = compute_follower_command(rover.pose, target);
                rover.follower_status = command.status;
                if command.status.arrived {
                    rover.follower_target = None;
                    rover.throttle = 0.0;
                    rover.steering = 0.0;
                } else {
                    rover.throttle = command.throttle;
                    rover.steering = command.steering;
                }
                (rover.throttle, rover.steering)
            } else {
                rover.follower_status = FollowerStatus::default();
                (rover.throttle, rover.steering)
            };

            let steering = steering.clamp(-1.0, 1.0);
            let throttle = throttle.clamp(-1.0, 1.0);

            let mut left = throttle;
            let mut right = throttle;

            let is_tank_drive = true;

            if is_tank_drive {
                left = throttle + steering;
                right = throttle - steering;
            } else {
                if steering > 0.0 {
                    right *= 1.0 - steering;
                } else if steering < 0.0 {
                    left *= 1.0 + steering;
                }
            }

            let left = left.clamp(-1.0, 1.0) * MAX_MOTOR_VELOCITY;
            let right = right.clamp(-1.0, 1.0) * MAX_MOTOR_VELOCITY;

            let reset_requested = rover.reset_requested;
            rover.reset_requested = false;
            (left, right, should_stop, reset_requested)
        };

        if reset_requested {
            chassis.reset_pose(Pose2d::default());
        }

        let left_front_result;
        let right_front_result;
        let left_back_result;
        let right_back_result;

        if should_stop {
            left_front.set_stop()?;
            right_front.set_stop()?;
            left_back.set_stop()?;
            right_back.set_stop()?;
            left_front_result = left_front.query();
            right_front_result = right_front.query();
            left_back_result = left_back.query();
            right_back_result = right_back.query();
        } else {
            left_front_result = left_front.set_position(velocity_command(
                left_velocity * CHASSIS_CONFIG.left_front_direction,
            ));
            right_front_result = right_front.set_position(velocity_command(
                right_velocity * CHASSIS_CONFIG.right_front_direction,
            ));
            left_back_result = left_back.set_position(velocity_command(
                left_velocity * CHASSIS_CONFIG.left_back_direction,
            ));
            right_back_result = right_back.set_position(velocity_command(
                right_velocity * CHASSIS_CONFIG.right_back_direction,
            ));
        }

        let mut rover = state.rover.lock().expect("rover state poisoned");
        match (
            &left_front_result,
            &right_front_result,
            &left_back_result,
            &right_back_result,
        ) {
            (
                Ok(left_front_feedback),
                Ok(right_front_feedback),
                Ok(left_back_feedback),
                Ok(right_back_feedback),
            ) => {
                let pose = chassis.update_from_four_motors(
                    left_front_feedback.position,
                    left_back_feedback.position,
                    right_front_feedback.position,
                    right_back_feedback.position,
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
                        id: CHASSIS_CONFIG.left_front_id,
                        position: left_front_feedback.position,
                        velocity: left_front_feedback.velocity,
                        fault: left_front_feedback.fault,
                    },
                    MotorTelemetry {
                        id: CHASSIS_CONFIG.right_front_id,
                        position: right_front_feedback.position,
                        velocity: right_front_feedback.velocity,
                        fault: right_front_feedback.fault,
                    },
                    MotorTelemetry {
                        id: CHASSIS_CONFIG.left_back_id,
                        position: left_back_feedback.position,
                        velocity: left_back_feedback.velocity,
                        fault: left_back_feedback.fault,
                    },
                    MotorTelemetry {
                        id: CHASSIS_CONFIG.right_back_id,
                        position: right_back_feedback.position,
                        velocity: right_back_feedback.velocity,
                        fault: right_back_feedback.fault,
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

pub(crate) fn read_motor_continuously(motor_id: u8) -> Result<(), moteus::Error> {
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
