//! Independent, supervised DepthAI capture. No SDK is needed to build or test.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Write};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct CameraConfig {
    pub name: String,
    pub mxid: String,
    pub rotate_180: bool,
}
impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            name: "OAK-D".into(),
            mxid: String::new(),
            rotate_180: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub python: String,
    pub fps: u32,
    pub cameras: [CameraConfig; 3],
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            python: "python3".into(),
            fps: 10,
            cameras: std::array::from_fn(|i| CameraConfig {
                name: format!("OAK-D {}", i + 1),
                ..Default::default()
            }),
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.python.trim().is_empty() || !(1..=15).contains(&self.fps) {
            return Err("OAK-D requires a Python interpreter and FPS between 1 and 15".into());
        }
        let mut ids = std::collections::HashSet::new();
        for camera in &self.cameras {
            if camera.name.trim().is_empty()
                || camera.name.len() > 80
                || camera.mxid.len() > 128
                || camera.mxid.trim() != camera.mxid
                || (!camera.mxid.is_empty() && !ids.insert(&camera.mxid))
            {
                return Err(
                    "OAK-D cameras require names and unique device IDs (or blank for discovery)"
                        .into(),
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Camera {
    pub mxid: String,
    pub connected: bool,
    pub updated_at: Option<f64>,
    pub error: Option<String>,
    pub min_mm: Option<u32>,
    pub avg_mm: Option<u32>,
    pub max_mm: Option<u32>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Snapshot {
    pub cameras: [Camera; 3],
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    State {
        cameras: Box<[Camera; 3]>,
    },
    Image {
        slot: usize,
        kind: String,
        jpeg: String,
        captured_at: f64,
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

pub type Images = [[Option<(Vec<u8>, Instant)>; 2]; 3];

pub struct Sidecar {
    child: Child,
    events: Receiver<Result<Event, String>>,
    reader: Option<std::thread::JoinHandle<()>>,
}
impl Sidecar {
    pub fn spawn(config: &Config, script: &std::path::Path) -> Result<Self, String> {
        config.validate()?;
        let project = script.parent().ok_or("OAK-D sidecar directory missing")?;
        if !project.join("pyproject.toml").is_file() || !project.join("uv.lock").is_file() {
            return Err(
                "OAK-D sidecar requires pyproject.toml and uv.lock beside its scripts".into(),
            );
        }
        let setup = Command::new("uv")
            .args([
                "venv",
                "--allow-existing",
                "--system-site-packages",
                "--no-managed-python",
                "--python",
            ])
            .arg(&config.python)
            .arg(project.join(".venv"))
            .stdout(Stdio::null())
            .status()
            .map_err(|e| format!("OAK-D requires uv: {e}"))?;
        if !setup.success() {
            return Err(format!("OAK-D Python environment setup failed: {setup}"));
        }
        let mut command = Command::new("uv");
        command
            .args(["run", "--project"])
            .arg(project)
            // Keep the runtime environment free of test-only packages so the
            // host NumPy/OpenCV ABI stays authoritative.
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
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let output = child.stdout.take().unwrap();
        let (tx, events) = mpsc::sync_channel(32);
        let reader = std::thread::spawn(move || {
            let mut output = BufReader::new(output);
            loop {
                let event = read_event(&mut output).map_err(|e| e.to_string());
                let failed = event.is_err();
                if tx.send(event).is_err() || failed {
                    break;
                }
            }
        });
        let process = Self {
            child,
            events,
            reader: Some(reader),
        };
        let mut input = process.child.stdin.as_ref().unwrap();
        serde_json::to_writer(&mut input, config).map_err(|e| e.to_string())?;
        input.write_all(b"\n").map_err(|e| e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        Ok(process)
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<Event>, String> {
        match self.events.recv_timeout(timeout) {
            Ok(event) => event.map(Some),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err("OAK-D sidecar exited".into()),
        }
    }
}
impl Drop for Sidecar {
    fn drop(&mut self) {
        let (_, empty) = mpsc::channel();
        drop(std::mem::replace(&mut self.events, empty));
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
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
        .take(2 * 1024 * 1024 + 1)
        .read_until(b'\n', &mut line)?;
    if line.is_empty() || line.len() > 2 * 1024 * 1024 || !line.ends_with(b"\n") {
        return Err(io::Error::other(
            "OAK-D sidecar closed or sent an oversized/incomplete message",
        ));
    }
    serde_json::from_slice(&line).map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_and_protocol_are_sdk_free() {
        Config::default().validate().unwrap();
        assert_eq!(Event::image_bytes("aGk=").unwrap(), b"hi");
        assert!(Event::image_bytes("!").is_err());
        let mut c = Config {
            fps: 16,
            ..Default::default()
        };
        assert!(c.validate().is_err());
        c.fps = 10;
        c.cameras[0].mxid = "abc".into();
        c.cameras[1].mxid = "abc".into();
        assert!(c.validate().is_err());
        assert!(matches!(
            read_event(&mut io::Cursor::new(
                b"{\"type\":\"error\",\"error\":\"missing SDK\"}\n"
            ))
            .unwrap(),
            Event::Error { .. }
        ));
        assert!(read_event(&mut io::Cursor::new(b"{}\n")).is_err());
        assert!(
            read_event(&mut io::Cursor::new(
                b"{\"type\":\"error\",\"error\":\"partial\"}"
            ))
            .is_err()
        );
    }

    #[test]
    fn fake_sidecar_receives_three_slots_and_is_reaped() {
        let dir = tempfile::tempdir().unwrap();
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sidecar");
        for name in ["pyproject.toml", "uv.lock"] {
            std::fs::copy(source.join(name), dir.path().join(name)).unwrap();
        }
        let script = dir.path().join("fake.py");
        std::fs::write(&script, "import json,sys,time\nc=json.loads(sys.stdin.readline())\nassert len(c['cameras'])==3\nprint(json.dumps({'type':'error','error':'three slots configured'}),flush=True)\ntime.sleep(60)\n").unwrap();
        let sidecar = Sidecar::spawn(&Config::default(), &script).unwrap();
        let pid = sidecar.child.id();
        let event = sidecar
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        assert!(matches!(event, Event::Error { error } if error == "three slots configured"));
        drop(sidecar);
        #[cfg(target_os = "linux")]
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }
}
