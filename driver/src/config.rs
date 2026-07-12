use std::fs;

use serde::Deserialize;

use crate::localizer::ChassisConfig;

#[derive(Debug, Deserialize)]
struct RawConfig {
    chassis: RawChassis,
    motors: RawMotors,
}

#[derive(Debug, Deserialize)]
struct RawChassis {
    wheel_radius_mm: f32,
    track_width_mm: f32,
    motor_rotations_per_wheel_rotation: f32,
}

#[derive(Debug, Deserialize)]
struct RawMotors {
    left_front: RawMotor,
    right_front: RawMotor,
    left_back: RawMotor,
    right_back: RawMotor,
}

#[derive(Debug, Deserialize)]
struct RawMotor {
    id: u8,
    direction: f32,
}

pub(crate) fn load_chassis_config(path: &str) -> Result<ChassisConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read config '{path}': {e}"))?;
    let raw: RawConfig = toml::from_str(&content).map_err(|e| format!("Failed to parse config: {e}"))?;

    Ok(ChassisConfig {
        wheel_radius_m: raw.chassis.wheel_radius_mm / 1000.0,
        track_width_m: raw.chassis.track_width_mm / 1000.0,
        motor_rotations_per_wheel_rotation: raw.chassis.motor_rotations_per_wheel_rotation,
        left_front_id: raw.motors.left_front.id,
        left_front_direction: raw.motors.left_front.direction,
        right_front_id: raw.motors.right_front.id,
        right_front_direction: raw.motors.right_front.direction,
        left_back_id: raw.motors.left_back.id,
        left_back_direction: raw.motors.left_back.direction,
        right_back_id: raw.motors.right_back.id,
        right_back_direction: raw.motors.right_back.direction,
    })
}
