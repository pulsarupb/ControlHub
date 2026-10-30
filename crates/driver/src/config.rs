use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub drive: roboteq::Config,
    pub kinova: KinovaConfig,
    pub rtk: rtk::Config,
    pub zed: zed::Config,
    pub gamepad: GamepadConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct KinovaConfig {
    pub enabled: bool,
    pub ip: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub linear_speed: f32,
    pub angular_speed: f32,
    pub joint_speed: f32,
}

impl Default for KinovaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ip: "192.168.1.10".into(),
            port: 10000,
            username: "admin".into(),
            password: String::new(),
            linear_speed: 0.02,
            angular_speed: 5.0,
            joint_speed: 5.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct GamepadConfig {
    pub dead_zone: f32,
    pub toggle_button: usize,
    pub deadman_button: usize,
    pub stop_button: usize,
    pub throttle_axis: usize,
    pub steering_axis: usize,
    pub arm_x_axis: usize,
    pub arm_y_axis: usize,
    pub arm_z_axis: usize,
    pub arm_yaw_axis: usize,
    pub pitch_up_button: usize,
    pub pitch_down_button: usize,
    pub roll_left_button: usize,
    pub roll_right_button: usize,
    pub gripper_open_button: usize,
    pub gripper_close_button: usize,
}

impl Default for GamepadConfig {
    fn default() -> Self {
        Self {
            dead_zone: 0.12,
            toggle_button: 3,
            deadman_button: 4,
            stop_button: 1,
            throttle_axis: 1,
            steering_axis: 2,
            arm_x_axis: 1,
            arm_y_axis: 0,
            arm_z_axis: 3,
            arm_yaw_axis: 2,
            pitch_up_button: 12,
            pitch_down_button: 13,
            roll_left_button: 14,
            roll_right_button: 15,
            gripper_open_button: 6,
            gripper_close_button: 7,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        self.drive.validate()?;
        self.rtk.validate()?;
        self.zed.validate()?;
        let k = &self.kinova;
        if k.ip.trim().is_empty()
            || k.port == 0
            || ![
                (k.linear_speed, 0.1),
                (k.angular_speed, 30.0),
                (k.joint_speed, 30.0),
            ]
            .iter()
            .all(|(v, max)| v.is_finite() && *v > 0.0 && v <= max)
        {
            return Err("invalid Kinova address or speed limits (max 0.1 m/s, 30 deg/s)".into());
        }
        let g = &self.gamepad;
        if !g.dead_zone.is_finite()
            || !(0.0..0.9).contains(&g.dead_zone)
            || [
                g.toggle_button,
                g.deadman_button,
                g.stop_button,
                g.pitch_up_button,
                g.pitch_down_button,
                g.roll_left_button,
                g.roll_right_button,
                g.gripper_open_button,
                g.gripper_close_button,
            ]
            .iter()
            .any(|v| *v > 31)
            || [
                g.throttle_axis,
                g.steering_axis,
                g.arm_x_axis,
                g.arm_y_axis,
                g.arm_z_axis,
                g.arm_yaw_axis,
            ]
            .iter()
            .any(|v| *v > 15)
            || g.toggle_button == g.stop_button
            || g.deadman_button == g.stop_button
            || g.toggle_button == g.deadman_button
        {
            return Err("invalid gamepad bindings/dead zone".into());
        }
        if self.rtk.enabled && self.drive.enabled && self.drive.ports.contains(&self.rtk.port) {
            return Err("RTK and motor controllers cannot share a serial path".into());
        }
        Ok(())
    }
}

pub fn load(path: &str) -> Result<Config, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let config: Config = serde_json::from_str(&text).map_err(|e| {
                format!("invalid configuration (back up and migrate old schema): {e}")
            })?;
            config.validate()?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn save(path: &str, config: &Config) -> Result<(), String> {
    use std::io::Write;
    config.validate()?;
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
    file.write_all(serde_json::to_string_pretty(config).unwrap().as_bytes())
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_disabled_and_roundtrip() {
        let c = Config::default();
        c.validate().unwrap();
        assert!(!c.drive.enabled && !c.kinova.enabled && !c.rtk.enabled && !c.zed.enabled);
        assert_eq!(
            serde_json::from_str::<Config>(&serde_json::to_string(&c).unwrap()).unwrap(),
            c
        );
    }
    #[test]
    fn reject_old_schema_and_shared_ports() {
        assert!(serde_json::from_str::<Config>(r#"{"chassis":{},"motors":{}}"#).is_err());
        let mut c = Config::default();
        c.drive.enabled = true;
        c.drive.ports = ["a".into(), "b".into()];
        c.rtk.enabled = true;
        c.rtk.port = "a".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn ramp_settings_are_integer_milliseconds_and_migrate_seconds() {
        let legacy: Config = serde_json::from_str(
            r#"{"drive":{"acceleration_time":0.125,"deceleration_time":0.25}}"#,
        )
        .unwrap();
        assert_eq!(legacy.drive.acceleration_time_ms, 125);
        assert_eq!(legacy.drive.deceleration_time_ms, 250);

        let serialized = serde_json::to_value(legacy).unwrap();
        assert_eq!(serialized["drive"]["acceleration_time_ms"], 125);
        assert_eq!(serialized["drive"]["deceleration_time_ms"], 250);
        assert!(serialized["drive"].get("acceleration_time").is_none());
    }
}
