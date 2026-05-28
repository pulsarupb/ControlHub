use moteus::{BlockingController, command::PositionCommand};
use std::env;
use std::time::Duration;

mod localizer;

use localizer::{Chassis, ChassisConfig};

const DRIVE_VELOCITY: f32 = 0.2;

const CHASSIS_CONFIG: ChassisConfig = ChassisConfig {
    wheel_radius_m: 21.59 / 2.0 / 100.0,
    track_width_m: 44.0 / 100.0,
    motor_rotations_per_wheel_rotation: 1.0,
    left_front_direction: 1.0,
    right_front_direction: -1.0,
    left_back_direction: -1.0,
    right_back_direction: 1.0,
};

fn main() -> Result<(), moteus::Error> {
    let args: Vec<String> = env::args().collect();
    if matches!(args.get(1).map(String::as_str), Some("read" | "calibrate")) {
        let Some(motor_id) = args.get(2).and_then(|value| value.parse::<u8>().ok()) else {
            println!("Usage: cargo run -p driver -- read <motor_id>");
            return Ok(());
        };

        return read_motor_continuously(motor_id);
    }

    let mut result1 = BlockingController::new(1);
    let mut ctrl1 = match result1 {
        Ok(ctrl) => ctrl,
        Err(err) => {
            println!("Error creating controller: {}", err);
            return Err(err);
        }
    };
    ctrl1.set_stop()?;

    let mut result2 = BlockingController::new(2);
    let mut ctrl2 = match result2 {
        Ok(ctrl) => ctrl,
        Err(err) => {
            println!("Error creating controller: {}", err);
            return Err(err);
        }
    };
    ctrl2.set_stop()?;

    let mut result3 = BlockingController::new(3);
    let mut ctrl3 = match result3 {
        Ok(ctrl) => ctrl,
        Err(err) => {
            println!("Error creating controller: {}", err);
            return Err(err);
        }
    };
    ctrl3.set_stop()?;

    let mut result4 = BlockingController::new(4);
    let mut ctrl4 = match result4 {
        Ok(ctrl) => ctrl,
        Err(err) => {
            println!("Error creating controller: {}", err);
            return Err(err);
        }
    };
    ctrl4.set_stop()?;

    let mut chassis = Chassis::new(CHASSIS_CONFIG);

    loop {
        let mut command1 = PositionCommand::new();
        command1.position = Some(f32::NAN);
        command1.velocity = Some(DRIVE_VELOCITY * CHASSIS_CONFIG.left_front_direction);
        command1.watchdog_timeout = Some(0.5);

        let mut command2 = PositionCommand::new();
        command2.position = Some(f32::NAN);
        command2.velocity = Some(DRIVE_VELOCITY * CHASSIS_CONFIG.right_back_direction);
        command2.watchdog_timeout = Some(0.5);

        let mut command3 = PositionCommand::new();
        command3.position = Some(f32::NAN);
        command3.velocity = Some(DRIVE_VELOCITY * CHASSIS_CONFIG.right_front_direction);
        command3.watchdog_timeout = Some(0.5);

        let mut command4 = PositionCommand::new();
        command4.position = Some(f32::NAN);
        command4.velocity = Some(DRIVE_VELOCITY * CHASSIS_CONFIG.left_back_direction);
        command4.watchdog_timeout = Some(0.5);

        let result1 = ctrl1.set_position(command1);
        let result2 = ctrl2.set_position(command2);
        let result3 = ctrl3.set_position(command3);
        let result4 = ctrl4.set_position(command4);

        if let (Ok(feedback1), Ok(feedback2), Ok(feedback3), Ok(feedback4)) =
            (&result1, &result2, &result3, &result4)
        {
            let pose = chassis.update_from_four_motors(
                feedback1.position,
                feedback4.position,
                feedback3.position,
                feedback2.position,
            );

            println!(
                "Pose - x={:.3}m y={:.3}m heading={:.1}deg",
                pose.x_m,
                pose.y_m,
                pose.heading_rad.to_degrees()
            );
        }

        match result1 {
            Ok(result) => {
                println!(
                    "Controller 1 - mode={:?} pos={:.4} vel={:.4} fault={}",
                    result.mode, result.position, result.velocity, result.fault
                );
            }
            Err(err) => {
                println!("Error sending position command to controller 1: {}", err);
            }
        }

        match result2 {
            Ok(result) => {
                println!(
                    "Controller 2 - mode={:?} pos={:.4} vel={:.4} fault={}",
                    result.mode, result.position, result.velocity, result.fault
                );
            }
            Err(err) => {
                println!("Error sending position command to controller 2: {}", err);
            }
        }

        match result3 {
            Ok(result) => {
                println!(
                    "Controller 3 - mode={:?} pos={:.4} vel={:.4} fault={}",
                    result.mode, result.position, result.velocity, result.fault
                );
            }
            Err(err) => {
                println!("Error sending position command to controller 3: {}", err);
            }
        }

        match result4 {
            Ok(result) => {
                println!(
                    "Controller 4 - mode={:?} pos={:.4} vel={:.4} fault={}",
                    result.mode, result.position, result.velocity, result.fault
                );
            }
            Err(err) => {
                println!("Error sending position command to controller 4: {}", err);
            }
        }

        std::thread::sleep(Duration::from_millis(20));
    }
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
