//! Independent, supervised DepthAI capture. No SDK is needed by the Rust server.
use crate::state::{AppState, Device, epoch};
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
enum Event {
    State {
        cameras: [Camera; 3],
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

pub type Images = [[Option<(Vec<u8>, Instant)>; 2]; 3];

struct Sidecar {
    child: Child,
    events: Receiver<Result<Event, String>>,
    reader: Option<std::thread::JoinHandle<()>>,
}
impl Sidecar {
    fn spawn(config: &Config, script: &std::path::Path) -> Result<Self, String> {
        let project = script.parent().ok_or("OAK-D sidecar directory missing")?;
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
            .args(["--locked", "--no-managed-python", "--python"])
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

pub fn run(state: AppState, script: std::path::PathBuf) {
    while !state
        .shutting_down
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        let config = state.config.lock().unwrap().oak.clone();
        {
            let mut status = state.status.lock().unwrap();
            status.oak = Device {
                enabled: config.enabled,
                simulated: state.simulate && config.enabled,
                ..Default::default()
            };
        }
        *state.oak_images.lock().unwrap() = Default::default();
        if !config.enabled {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        if let Err(error) = session(&state, &config, &script) {
            let mut status = state.status.lock().unwrap();
            status.oak.fail(error);
            for camera in &mut status.oak.data.cameras {
                camera.connected = false;
            }
        }
        *state.oak_images.lock().unwrap() = Default::default();
        std::thread::sleep(Duration::from_secs(1));
    }
}
fn session(state: &AppState, config: &Config, script: &std::path::Path) -> Result<(), String> {
    if state.simulate {
        while !state
            .shutting_down
            .load(std::sync::atomic::Ordering::SeqCst)
            && state.config.lock().unwrap().oak == *config
        {
            state.status.lock().unwrap().oak.update(Snapshot::default());
            std::thread::sleep(Duration::from_millis(100));
        }
        return Ok(());
    }
    let sidecar = Sidecar::spawn(config, script)?;
    let mut last_event = Instant::now();
    while !state
        .shutting_down
        .load(std::sync::atomic::Ordering::SeqCst)
        && state.config.lock().unwrap().oak == *config
    {
        match sidecar.events.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                last_event = Instant::now();
                match event {
                    Event::State { mut cameras } => {
                        for (slot, camera) in cameras.iter_mut().enumerate() {
                            camera.connected &= camera
                                .updated_at
                                .is_some_and(|t| (0.0..3.0).contains(&(epoch() - t)));
                            if !camera.connected {
                                state.oak_images.lock().unwrap()[slot] = Default::default();
                            }
                        }
                        state
                            .status
                            .lock()
                            .unwrap()
                            .oak
                            .update(Snapshot { cameras });
                    }
                    Event::Image {
                        slot,
                        kind,
                        jpeg,
                        captured_at,
                    } => {
                        let index = match kind.as_str() {
                            "color" => 0,
                            "depth" => 1,
                            _ => return Err("invalid OAK-D image kind".into()),
                        };
                        if slot >= 3 {
                            return Err("invalid OAK-D camera slot".into());
                        }
                        let age = epoch() - captured_at;
                        if (0.0..3.0).contains(&age) {
                            let bytes =
                                zed::Event::image_bytes(&jpeg).map_err(|e| e.to_string())?;
                            state.oak_images.lock().unwrap()[slot][index] =
                                Some((bytes, Instant::now() - Duration::from_secs_f64(age)));
                        }
                    }
                    Event::Error { error } => return Err(error),
                }
            }
            Ok(Err(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err("OAK-D sidecar exited".into()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if last_event.elapsed() > Duration::from_secs(30) {
            return Err("OAK-D sidecar stalled".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn config_and_protocol_are_sdk_free() {
        Config::default().validate().unwrap();
        let mut c = Config::default();
        c.fps = 16;
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
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../zed/sidecar");
        for name in ["pyproject.toml", "uv.lock"] {
            std::fs::copy(source.join(name), dir.path().join(name)).unwrap();
        }
        let script = dir.path().join("fake.py");
        std::fs::write(&script, "import json,sys,time\nc=json.loads(sys.stdin.readline())\nassert len(c['cameras'])==3\nprint(json.dumps({'type':'error','error':'three slots configured'}),flush=True)\ntime.sleep(60)\n").unwrap();
        let sidecar = Sidecar::spawn(&Config::default(), &script).unwrap();
        let pid = sidecar.child.id();
        let event = sidecar
            .events
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        assert!(matches!(event, Event::Error { error } if error == "three slots configured"));
        drop(sidecar);
        #[cfg(target_os = "linux")]
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    fn simulation_does_not_spawn_python_or_fabricate_frames() {
        let mut config = crate::config::Config::default();
        config.oak.enabled = true;
        let (state, _) = AppState::new(config, String::new(), true);
        let worker_state = state.clone();
        let worker = std::thread::spawn(move || {
            let config = worker_state.config.lock().unwrap().oak.clone();
            session(
                &worker_state,
                &config,
                std::path::Path::new("/nonexistent/oak.py"),
            )
        });
        let deadline = Instant::now() + Duration::from_secs(3);
        while !state.status.lock().unwrap().oak.connected {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        state
            .shutting_down
            .store(true, std::sync::atomic::Ordering::SeqCst);
        worker.join().unwrap().unwrap();
        assert!(
            state
                .oak_images
                .lock()
                .unwrap()
                .iter()
                .flatten()
                .all(Option::is_none)
        );
        assert!(
            state
                .status
                .lock()
                .unwrap()
                .oak
                .data
                .cameras
                .iter()
                .all(|c| !c.connected)
        );
    }
}
