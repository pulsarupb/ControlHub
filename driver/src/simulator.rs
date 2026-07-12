use crate::localizer::{Chassis, ChassisConfig, Pose2d};
use crate::motor::{compute_tank_drive, TankDriveCommand, MOTOR_PERIOD};
use crate::state::{AppState, MotorTelemetry};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct SimulatedMotor {
    position: f32,
    velocity: f32,
    fault: i8,
}

impl SimulatedMotor {
    fn new() -> Self {
        Self { position: 0.0, velocity: 0.0, fault: 0 }
    }

    fn set_velocity(&mut self, vel: f32) {
        self.velocity = vel;
    }

    fn set_stop(&mut self) {
        self.velocity = 0.0;
    }

    fn tick(&mut self, dt: f32) {
        self.position += self.velocity * dt;
    }

    #[allow(dead_code)]
    fn query(&self) -> (f32, f32, i8) {
        (self.position, self.velocity, self.fault)
    }
}

pub(crate) fn run_simulated_motor_loop(
    state: AppState,
    shared_config: Arc<Mutex<ChassisConfig>>,
    restart_flag: Arc<AtomicBool>,
) {
    if let Err(err) = run_simulated_loop_inner(state.clone(), shared_config, restart_flag) {
        let mut rover = state.rover.lock().expect("rover state poisoned");
        rover.emergency_stop = true;
        rover.last_error = Some(err.clone());
        eprintln!("Simulated motor loop stopped: {err}");
    }
}

fn run_simulated_loop_inner(
    state: AppState,
    shared_config: Arc<Mutex<ChassisConfig>>,
    restart_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let mut left_front = SimulatedMotor::new();
    let mut right_front = SimulatedMotor::new();
    let mut left_back = SimulatedMotor::new();
    let mut right_back = SimulatedMotor::new();

    let initial_config = shared_config.lock().map_err(|e| e.to_string())?;
    let mut chassis = Chassis::new(*initial_config);
    drop(initial_config);
    let mut next_tick = Instant::now();

    loop {
        if restart_flag.load(Ordering::SeqCst) {
            return Ok(());
        }

        next_tick += MOTOR_PERIOD;

        let config = shared_config.lock().map_err(|e| e.to_string())?;
        let max_velocity = config.max_velocity;
        let lf_dir = config.left_front_direction;
        let rf_dir = config.right_front_direction;
        let lb_dir = config.left_back_direction;
        let rb_dir = config.right_back_direction;
        chassis.config = *config;
        drop(config);

        let cmd: TankDriveCommand = {
            let mut rover = state.rover.lock().expect("rover state poisoned");
            compute_tank_drive(&mut rover, max_velocity)
        };

        if cmd.reset_requested {
            chassis.reset_pose(Pose2d::default());
        }

        if cmd.should_stop {
            left_front.set_stop();
            right_front.set_stop();
            left_back.set_stop();
            right_back.set_stop();
        } else {
            left_front.set_velocity(cmd.left_velocity * lf_dir);
            right_front.set_velocity(cmd.right_velocity * rf_dir);
            left_back.set_velocity(cmd.left_velocity * lb_dir);
            right_back.set_velocity(cmd.right_velocity * rb_dir);
        }

        let dt = MOTOR_PERIOD.as_secs_f32();
        left_front.tick(dt);
        right_front.tick(dt);
        left_back.tick(dt);
        right_back.tick(dt);

        let lf = left_front.query();
        let rf = right_front.query();
        let lb = left_back.query();
        let rb = right_back.query();

        let mut rover = state.rover.lock().expect("rover state poisoned");
        let pose = chassis.update_from_four_motors(lf.0, lb.0, rf.0, rb.0);
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
            MotorTelemetry { id: chassis.config.left_front_id, position: lf.0, velocity: lf.1, fault: lf.2 },
            MotorTelemetry { id: chassis.config.right_front_id, position: rf.0, velocity: rf.1, fault: rf.2 },
            MotorTelemetry { id: chassis.config.left_back_id, position: lb.0, velocity: lb.1, fault: lb.2 },
            MotorTelemetry { id: chassis.config.right_back_id, position: rb.0, velocity: rb.1, fault: rb.2 },
        ];
        rover.last_error = None;
        drop(rover);

        let now = Instant::now();
        if next_tick > now {
            std::thread::sleep(next_tick - now);
        } else {
            next_tick = now;
        }
    }
}
