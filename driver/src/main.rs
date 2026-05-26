use moteus::{
    BlockingController, TransportOptions,
    command::{self, PositionCommand},
};
use std::time::Duration;
fn main() -> Result<(), moteus::Error> {
    // let mut found = Vec::new();
    // for id in 0u8..=6 {
    //     print!("Trying ID {}... ", id);
    //     match BlockingController::new(id) {
    //         Ok(mut ctrl) => match ctrl.set_stop() {
    //             Ok(result) => {
    //                 println!(
    //                     "FOUND mode={:?} pos={:.4} vel={:.4} fault={}",
    //                     result.mode, result.position, result.velocity, result.fault
    //                 );

    //                 found.push(id);
    //             }
    //             Err(err) => {
    //                 println!("no reply ({})", err);
    //             }
    //         },
    //         Err(err) => {
    //             println!("transport error ({})", err);
    //         }
    //     }
    // }
    // println!();
    // println!("Found moteus IDs: {:?}", found);

    let mut result1 = BlockingController::new(0);
    let mut ctrl1 = match result1 {
        Ok(ctrl) => ctrl,
        Err(err) => {
            println!("Error creating controller: {}", err);
            return Err(err);
        }
    };
    ctrl1.set_stop()?;

    let mut result2 = BlockingController::new(1);
    let mut ctrl2 = match result2 {
        Ok(ctrl) => ctrl,
        Err(err) => {
            println!("Error creating controller: {}", err);
            return Err(err);
        }
    };

    ctrl2.set_stop()?;

    let mut result3 = BlockingController::new(2);
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

    loop {
        let mut command1 = PositionCommand::new();
        command1.velocity = Some(1.0);
        command1.watchdog_timeout = Some(0.5);

        let mut command2 = PositionCommand::new();
        command2.velocity = Some(1.0);
        command2.watchdog_timeout = Some(0.5);

        let mut command3 = PositionCommand::new();
        command3.velocity = Some(1.0);
        command3.watchdog_timeout = Some(0.5);

        let mut command4 = PositionCommand::new();
        command4.velocity = Some(1.0);
        command4.watchdog_timeout = Some(0.5);

        let result1 = ctrl1.set_position(command1);
        let result2 = ctrl2.set_position(command2);
        let result3 = ctrl3.set_position(command3);
        let result4 = ctrl4.set_position(command4);

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
