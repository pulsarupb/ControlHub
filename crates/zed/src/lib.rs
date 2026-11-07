//! Supervised pyzed sidecar. No SDK required to build or test this library.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::JoinHandle;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub python: String,
    pub settings: Value,
    pub mount_configured: bool,
    /// Camera pose in rover base frame, meters / roll-pitch-yaw degrees.
    pub mount_translation: [f64; 3],
    pub mount_rotation_deg: [f64; 3],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            python: "python3".into(),
            settings: serde_json::from_str(include_str!("../sidecar/defaults.json")).unwrap(),
            mount_configured: false,
            mount_translation: [0.0; 3],
            mount_rotation_deg: [0.0; 3],
        }
    }
}

impl Config {
    /// SDK quaternion is [x,y,z,w]; mount is camera-to-base Rz(yaw)Ry(pitch)Rx(roll).
    pub fn base_yaw(&self, orientation: [f64; 4]) -> Option<f64> {
        if !self.mount_configured || !orientation.iter().all(|v| v.is_finite()) {
            return None;
        }
        let norm = orientation.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !(0.9..=1.1).contains(&norm) {
            return None;
        }
        let [x, y, z, w] = orientation.map(|v| v / norm);
        let [r, p, h] = self.mount_rotation_deg.map(|v| v.to_radians() / 2.0);
        let (sr, cr) = r.sin_cos();
        let (sp, cp) = p.sin_cos();
        let (sh, ch) = h.sin_cos();
        let a = -(sr * cp * ch - cr * sp * sh);
        let b = -(cr * sp * ch + sr * cp * sh);
        let c = -(cr * cp * sh - sr * sp * ch);
        let d = cr * cp * ch + sr * sp * sh;
        let bx = w * a + x * d + y * c - z * b;
        let by = w * b - x * c + y * d + z * a;
        let bz = w * c + x * b - y * a + z * d;
        let bw = w * d - x * a - y * b - z * c;
        let forward_x = 1.0 - 2.0 * (by * by + bz * bz);
        let forward_y = 2.0 * (bx * by + bw * bz);
        if forward_x.hypot(forward_y) < 0.1 {
            return None;
        }
        Some(forward_y.atan2(forward_x))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.python.is_empty()
            || !self
                .mount_translation
                .iter()
                .chain(&self.mount_rotation_deg)
                .all(|v| v.is_finite())
        {
            return Err("ZED requires a Python interpreter and finite mounting transform".into());
        }
        let defaults = Self::default().settings;
        if self.settings.to_string().len() > 65536 {
            return Err("ZED settings exceed the sidecar message limit".into());
        }
        for section in ["init", "runtime", "tracking", "mapping"] {
            if !self.settings[section].is_object() {
                return Err(format!("ZED requires {section} settings"));
            }
            let expected = defaults[section].as_object().unwrap();
            let actual = self.settings[section].as_object().unwrap();
            if actual.keys().any(|key| !expected.contains_key(key)) {
                return Err(format!("unknown ZED {section} setting"));
            }
            for (key, default) in expected {
                let value = &self.settings[section][key];
                if !(default.is_boolean() && value.is_boolean()
                    || default.is_number() && value.is_number()
                    || default.is_string() && value.is_string())
                {
                    return Err(format!("missing or incorrectly typed ZED {section}.{key}"));
                }
            }
        }
        if self.settings["init"]["coordinate_units"] != "METER"
            || self.settings["init"]["coordinate_system"] != "RIGHT_HANDED_Z_UP"
        {
            return Err("ZED API requires METER and RIGHT_HANDED_Z_UP".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pose {
    pub translation: [f64; 3],
    pub orientation: [f64; 4],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub camera_ready: bool,
    pub camera_model: String,
    pub serial: String,
    pub tracking_state: String,
    pub mapping_state: String,
    pub fps: f64,
    pub pose: Option<Pose>,
    pub path: Vec<Pose>,
    pub imu: Option<Value>,
    pub points: Vec<[f32; 3]>,
    pub colors: Vec<[u8; 3]>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MeshData {
    pub vertices: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub colors: Vec<[u8; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub texture: Option<String>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct MeshSnapshot {
    pub generating: bool,
    pub generated_at: Option<f64>,
    pub mesh: Option<MeshData>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Mesh {
        mesh: Option<MeshData>,
        error: Option<String>,
    },
    Odometry {
        orientation: Option<[f64; 4]>,
        timestamp: f64,
        captured_at: f64,
        epoch: u64,
    },
    State {
        state: Box<Snapshot>,
    },
    Image {
        kind: String,
        jpeg: String,
    },
    Error {
        error: String,
    },
}

impl Event {
    pub fn image_bytes(jpeg: &str) -> io::Result<Vec<u8>> {
        STANDARD.decode(jpeg).map_err(io::Error::other)
    }
}

pub struct Sidecar {
    child: Child,
    input: ChildStdin,
    events: Receiver<io::Result<Event>>,
    reader: Option<JoinHandle<()>>,
}

impl Sidecar {
    /// `script` is explicit so applications can install sidecar files alongside
    /// their executable, rather than depending on the original source repository.
    pub fn spawn(config: &Config, script: &std::path::Path) -> io::Result<Self> {
        config.validate().map_err(io::Error::other)?;
        let project = script
            .parent()
            .ok_or_else(|| io::Error::other("ZED sidecar script has no project directory"))?;
        if !project.join("pyproject.toml").is_file() || !project.join("uv.lock").is_file() {
            return Err(io::Error::other(
                "ZED sidecar requires pyproject.toml and uv.lock beside its scripts",
            ));
        }

        let environment = project.join(".venv");
        let setup = Command::new("uv")
            .args([
                "venv",
                "--allow-existing",
                "--system-site-packages",
                "--no-managed-python",
                "--python",
            ])
            .arg(&config.python)
            .arg(&environment)
            .stdout(Stdio::null())
            .status()
            .map_err(|error| {
                io::Error::other(format!("uv is required to run the ZED sidecar: {error}"))
            })?;
        if !setup.success() {
            return Err(io::Error::other(format!(
                "uv failed to prepare the ZED sidecar environment: {setup}"
            )));
        }

        let mut command = Command::new("uv");
        command
            .args(["run", "--project"])
            .arg(project)
            // Runtime must not install the test-only NumPy: it would shadow the
            // system NumPy that the host OpenCV/pyzed bindings were built against.
            .args([
                "--locked",
                "--no-dev",
                "--exact",
                "--no-managed-python",
                "--python",
            ])
            .arg(&config.python)
            .args(["--", "python", "-u"])
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command.spawn()?;
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (tx, events) = mpsc::sync_channel(8);
        let reader = std::thread::spawn(move || {
            let mut output = BufReader::new(output);
            loop {
                let event = read_event(&mut output);
                let failed = event.is_err();
                if failed {
                    let _ = tx.send(event);
                    break;
                }
                // Video/map consumers never backpressure camera capture.
                if matches!(tx.try_send(event), Err(mpsc::TrySendError::Disconnected(_))) {
                    break;
                }
            }
        });
        let mut process = Self {
            child,
            input,
            events,
            reader: Some(reader),
        };
        process.send(&serde_json::json!({"type": "configure", "settings": config.settings}))?;
        Ok(process)
    }

    pub fn send(&mut self, command: &Value) -> io::Result<()> {
        serde_json::to_writer(&mut self.input, command)?;
        self.input.write_all(b"\n")?;
        self.input.flush()
    }

    pub fn poll(&mut self) -> io::Result<Option<Event>> {
        match self.events.try_recv() {
            Ok(event) => event.map(Some),
            Err(TryRecvError::Disconnected) => Err(io::Error::other(
                "ZED sidecar exited; inspect Python/SDK errors",
            )),
            Err(TryRecvError::Empty) => {
                if let Some(status) = self.child.try_wait()? {
                    return Err(io::Error::other(format!("ZED sidecar exited: {status}")));
                }
                Ok(None)
            }
        }
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        // Disconnect receiver before joining: an error sender may be waiting.
        let (_, empty) = mpsc::channel();
        drop(std::mem::replace(&mut self.events, empty));
        #[cfg(unix)]
        {
            // uv is a launcher process; kill its group so Python cannot outlive it.
            unsafe {
                libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
            }
        }
        #[cfg(not(unix))]
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn read_event(reader: &mut impl BufRead) -> io::Result<Event> {
    let mut line = Vec::new();
    reader
        .take(8 * 1024 * 1024 + 1)
        .read_until(b'\n', &mut line)?;
    if line.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "sidecar stdout closed",
        ));
    }
    if line.len() > 8 * 1024 * 1024 || !line.ends_with(b"\n") {
        return Err(io::Error::other("oversized or incomplete sidecar message"));
    }
    serde_json::from_slice(&line).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mesh_protocol_accepts_indexed_textured_geometry() {
        let message = serde_json::json!({
            "type": "mesh", "error": null,
            "mesh": {"vertices": [[0,0,0], [1,0,0], [0,1,0]],
                "triangles": [[0,1,2]], "colors": [[128,128,128], [128,128,128], [128,128,128]],
                "uv": [[0,0], [1,0], [0,1]], "texture": "data:image/jpeg;base64,anBlZw==",
                "truncated": false}
        });
        let Event::Mesh {
            mesh: Some(mesh),
            error: None,
        } = serde_json::from_value(message).unwrap()
        else {
            panic!("expected textured mesh event");
        };
        assert_eq!(mesh.triangles, vec![[0, 1, 2]]);
        assert_eq!(mesh.uv.len(), mesh.vertices.len());
        assert!(mesh.texture.is_some());
    }

    #[test]
    fn sdk_free_config_and_protocol() {
        Config::default().validate().unwrap();
        assert!(matches!(
            read_event(&mut io::Cursor::new(
                b"{\"type\":\"error\",\"error\":\"missing SDK\"}\n"
            ))
            .unwrap(),
            Event::Error { .. }
        ));
        assert!(read_event(&mut io::Cursor::new(b"not json\n")).is_err());
        assert!(Event::image_bytes("!").is_err());
    }
    #[test]
    fn disallows_coordinate_contract_changes() {
        let mut c = Config::default();
        c.settings["init"]["coordinate_units"] = "MILLIMETER".into();
        assert!(c.validate().is_err());
        let mut c = Config::default();
        c.settings["mapping"]["enabled"] = "true".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn yaw_uses_verified_mount_and_normalized_sdk_quaternion() {
        let mut c = Config::default();
        assert!(c.base_yaw([0.0, 0.0, 0.0, 1.0]).is_none());
        c.mount_configured = true;
        assert_eq!(c.base_yaw([0.0; 4]), None);
        assert_eq!(c.base_yaw([f64::NAN, 0.0, 0.0, 1.0]), None);
        let h = std::f64::consts::FRAC_PI_4;
        assert!((c.base_yaw([0.0, 0.0, h.sin(), h.cos()]).unwrap() - 2.0 * h).abs() < 1e-10);
        c.mount_rotation_deg = [0.0, 0.0, 90.0];
        assert!(c.base_yaw([0.0, 0.0, h.sin(), h.cos()]).unwrap().abs() < 1e-10);
        // Camera rolled 90 degrees in base: removing full mount rotation matters.
        c.mount_rotation_deg = [90.0, 0.0, 0.0];
        assert!(c.base_yaw([h.sin(), 0.0, 0.0, h.cos()]).unwrap().abs() < 1e-10);
    }

    #[test]
    fn odometry_protocol_is_sdk_free() {
        let event: Event = serde_json::from_str(r#"{"type":"odometry","orientation":[0,0,0,1],"timestamp":1.25,"captured_at":2.0,"epoch":3}"#).unwrap();
        assert!(matches!(event, Event::Odometry { epoch: 3, .. }));
    }

    #[test]
    fn fake_sidecar_receives_config_and_is_reaped() {
        let directory = tempfile::tempdir().unwrap();
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sidecar");
        for name in ["pyproject.toml", "uv.lock"] {
            std::fs::copy(source.join(name), directory.path().join(name)).unwrap();
        }
        let script = directory.path().join("fake.py");
        std::fs::write(
            &script,
            "import json,sys,time\nassert json.loads(sys.stdin.readline())['type']=='configure'\nprint(json.dumps({'type':'error','error':sys.prefix}),flush=True)\ntime.sleep(60)\n",
        )
        .unwrap();
        let config = Config {
            python: "python3".into(),
            ..Config::default()
        };
        let mut sidecar = Sidecar::spawn(&config, &script).unwrap();
        let pid = sidecar.child.id();
        let expected_prefix = directory
            .path()
            .join(".venv")
            .to_string_lossy()
            .into_owned();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if let Some(event) = sidecar.poll().unwrap() {
                assert!(matches!(event, Event::Error { error } if error == expected_prefix));
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "fake sidecar did not answer"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        drop(sidecar);
        #[cfg(target_os = "linux")]
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        #[cfg(not(target_os = "linux"))]
        let _ = pid;
    }
}
