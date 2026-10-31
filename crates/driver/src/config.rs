use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub drive: roboteq::Config,
    pub kinova: KinovaConfig,
    pub rtk: rtk::Config,
    pub zed: zed::Config,
    pub oak: crate::oak::Config,
    pub gamepad: GamepadConfig,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ConfigSecrets {
    kinova_password: String,
    rtk_ntrip_password: String,
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
        self.oak.validate()?;
        self.drive.validate()?;
        self.rtk.validate()?;
        self.zed.validate()?;
        if self.drive.yaw_control.enabled && (!self.zed.enabled || !self.zed.mount_configured) {
            return Err(
                "yaw control requires enabled ZED and a verified camera mounting transform".into(),
            );
        }
        if self.drive.yaw_control.enabled
            && (self.zed.settings["tracking"]["enable_area_memory"] != false
                || self.zed.settings["tracking"]["set_as_static"] != false)
        {
            return Err("yaw control requires enable_area_memory=false and set_as_static=false for continuous local odometry".into());
        }
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
    let config_path = Path::new(path);
    let mut config = match std::fs::read_to_string(config_path) {
        Ok(text) => {
            let config: Config = serde_json::from_str(&text).map_err(|e| {
                format!("invalid configuration (back up and migrate old schema): {e}")
            })?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e.to_string()),
    }?;

    let legacy_has_secrets =
        !config.kinova.password.is_empty() || !config.rtk.ntrip.password.is_empty();
    let secrets_path = secrets_path(config_path);
    match std::fs::read_to_string(&secrets_path) {
        Ok(text) => {
            let secrets: ConfigSecrets = serde_json::from_str(&text)
                .map_err(|e| format!("invalid credentials file: {e}"))?;
            config.kinova.password = secrets.kinova_password;
            config.rtk.ntrip.password = secrets.rtk_ntrip_password;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    config.validate()?;

    // Migrate older configs that stored credentials inline. The regular config
    // is rewritten without secrets, while the ignored sidecar stays mode 0600.
    if legacy_has_secrets {
        save(path, &config)?;
    }
    Ok(config)
}

pub fn save(path: &str, config: &Config) -> Result<(), String> {
    config.validate()?;
    let path = Path::new(path);
    let secrets = ConfigSecrets {
        kinova_password: config.kinova.password.clone(),
        rtk_ntrip_password: config.rtk.ntrip.password.clone(),
    };
    atomic_write(
        &secrets_path(path),
        serde_json::to_string_pretty(&secrets)
            .map_err(|e| e.to_string())?
            .as_bytes(),
        0o600,
    )?;

    let mut public_config = config.clone();
    public_config.kinova.password.clear();
    public_config.rtk.ntrip.password.clear();
    atomic_write(
        path,
        serde_json::to_string_pretty(&public_config)
            .map_err(|e| e.to_string())?
            .as_bytes(),
        0o644,
    )
}

fn secrets_path(path: &Path) -> PathBuf {
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    path.with_file_name(format!("{name}.secrets.json"))
}

fn atomic_write(path: &Path, contents: &[u8], mode: u32) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
    if let Some(parent) = parent {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    let owner = std::fs::metadata(path)
        .ok()
        .or_else(|| parent.and_then(|parent| std::fs::metadata(parent).ok()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|e| e.to_string())?;
    file.write_all(contents).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if let Some(owner) = owner.filter(|owner| {
            owner.uid() != unsafe { libc::geteuid() } || owner.gid() != unsafe { libc::getegid() }
        }) {
            let temporary_path =
                CString::new(temporary.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
            // Atomic rename otherwise changes a bind-mounted config to root:root
            // whenever the privileged container saves settings.
            let result = unsafe { libc::chown(temporary_path.as_ptr(), owner.uid(), owner.gid()) };
            if result != 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        file.set_permissions(std::fs::Permissions::from_mode(mode))
            .map_err(|e| e.to_string())?;
    }

    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_control_requires_measured_continuous_tracking_and_mount() {
        let mut c = Config::default();
        c.drive.yaw_control.enabled = true;
        assert!(c.validate().is_err());
        c.zed.enabled = true;
        assert!(c.validate().is_err());
        c.zed.mount_configured = true;
        assert!(c.validate().is_err()); // default area memory permits loop-closure jumps
        c.zed.settings["tracking"]["enable_area_memory"] = false.into();
        assert!(c.validate().is_ok());
        c.zed.settings["tracking"]["set_as_static"] = true.into();
        assert!(c.validate().is_err());
    }
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
    fn config_commit_file_redacts_credentials_and_keeps_private_sidecar() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rover16.json");
        let mut config = Config::default();
        config.kinova.password = "arm-secret".into();
        config.rtk.ntrip.password = "caster-secret".into();

        save(path.to_str().unwrap(), &config).unwrap();
        let public: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(public["kinova"]["password"], "");
        assert_eq!(public["rtk"]["ntrip"]["password"], "");
        assert!(!public.to_string().contains("secret"));
        assert_eq!(load(path.to_str().unwrap()).unwrap(), config);

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let secrets = std::fs::metadata(secrets_path(&path)).unwrap();
            assert_eq!(secrets.mode() & 0o777, 0o600);
            let config_metadata = std::fs::metadata(path).unwrap();
            assert_eq!(config_metadata.mode() & 0o777, 0o644);
            assert_eq!(
                config_metadata.uid(),
                std::fs::metadata(dir.path()).unwrap().uid()
            );
        }
    }

    #[test]
    fn legacy_inline_credentials_are_migrated_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rover16.json");
        let mut legacy = Config::default();
        legacy.kinova.password = "legacy-secret".into();
        std::fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();

        assert_eq!(load(path.to_str().unwrap()).unwrap(), legacy);
        let public: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(public["kinova"]["password"], "");
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
