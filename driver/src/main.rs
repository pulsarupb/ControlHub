use moteus::{
    BlockingController, TransportOptions,
    command::{self, PositionCommand},
};
use std::time::Duration;

mod localizer;

use localizer::{Chassis, ChassisConfig};

const CHASSIS_CONFIG: ChassisConfig = ChassisConfig {
    wheel_radius_m: 0.0,
    track_width_m: 0.0,
    motor_rotations_per_wheel_rotation: 1.0,
    left_encoder_sign: 1.0,
    right_encoder_sign: 1.0,
};

fn main() -> Result<(), moteus::Error> {
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
        command1.velocity = Some(5.0);
        command1.watchdog_timeout = Some(0.5);

        let mut command2 = PositionCommand::new();
        command2.position = Some(f32::NAN);
        command2.velocity = Some(5.0);
        command2.watchdog_timeout = Some(0.5);

        let mut command3 = PositionCommand::new();
        command3.position = Some(f32::NAN);
        command3.velocity = Some(5.0);
        command3.watchdog_timeout = Some(0.5);

        let mut command4 = PositionCommand::new();
        command4.position = Some(f32::NAN);
        command4.velocity = Some(5.0);
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
                feedback2.position,
                feedback3.position,
                feedback4.position,
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
