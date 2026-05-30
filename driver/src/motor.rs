use crate::localizer::{Chassis, ChassisConfig, Pose2d};
use crate::state::{AppState, MotorTelemetry};
use moteus::{BlockingController, command::PositionCommand};
use std::time::{Duration, Instant};

const MAX_MOTOR_VELOCITY: f32 = 1.0;
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

pub(crate) fn run_motor_loop(state: AppState) {
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
                    MotorTelemetry::from_feedback(1, feedback1),
                    MotorTelemetry::from_feedback(2, feedback2),
                    MotorTelemetry::from_feedback(3, feedback3),
                    MotorTelemetry::from_feedback(4, feedback4),
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
