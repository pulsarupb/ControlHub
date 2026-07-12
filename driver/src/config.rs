use std::fs;

use serde::{Deserialize, Serialize};

use crate::localizer::ChassisConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct RawConfig {
    pub chassis: RawChassis,
    pub motors: RawMotors,
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            chassis: RawChassis::default(),
            motors: RawMotors::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawChassis {
    pub wheel_radius_mm: f32,
    pub track_width_mm: f32,
    pub motor_rotations_per_wheel_rotation: f32,
    #[serde(default = "default_max_velocity")]
    pub max_velocity: f32,
}

impl Default for RawChassis {
    fn default() -> Self {
        Self {
            wheel_radius_mm: 107.95,
            track_width_mm: 440.0,
            motor_rotations_per_wheel_rotation: 1.0,
            max_velocity: default_max_velocity(),
        }
    }
}

fn default_max_velocity() -> f32 {
    2.0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawMotors {
    pub left_front: RawMotor,
    pub right_front: RawMotor,
    pub left_back: RawMotor,
    pub right_back: RawMotor,
}

impl Default for RawMotors {
    fn default() -> Self {
        Self {
            left_front: RawMotor { id: 1, direction: 1.0 },
            right_front: RawMotor { id: 3, direction: -1.0 },
            left_back: RawMotor { id: 4, direction: -1.0 },
            right_back: RawMotor { id: 2, direction: 1.0 },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawMotor {
    pub id: u8,
    pub direction: f32,
}

pub fn load_raw_config(path: &str) -> Result<RawConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config '{path}': {e}"))?;
    let raw: RawConfig =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {e}"))?;
    Ok(raw)
}

pub fn save_config(path: &str, config: &RawConfig) -> Result<(), String> {
    let parent = std::path::Path::new(path).parent();
    if let Some(dir) = parent {
        fs::create_dir_all(dir).map_err(|e| format!("Failed to create config directory: {e}"))?;
    }
    let content =
        serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize config: {e}"))?;
    fs::write(path, content).map_err(|e| format!("Failed to write config '{path}': {e}"))?;
    Ok(())
}

pub fn raw_to_chassis_config(raw: &RawConfig) -> ChassisConfig {
    ChassisConfig {
        wheel_radius_m: raw.chassis.wheel_radius_mm / 1000.0,
        track_width_m: raw.chassis.track_width_mm / 1000.0,
        motor_rotations_per_wheel_rotation: raw.chassis.motor_rotations_per_wheel_rotation,
        max_velocity: raw.chassis.max_velocity,
        left_front_id: raw.motors.left_front.id,
        left_front_direction: raw.motors.left_front.direction,
        right_front_id: raw.motors.right_front.id,
        right_front_direction: raw.motors.right_front.direction,
        left_back_id: raw.motors.left_back.id,
        left_back_direction: raw.motors.left_back.direction,
        right_back_id: raw.motors.right_back.id,
        right_back_direction: raw.motors.right_back.direction,
    }
}
