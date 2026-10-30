//! Supervised pyzed sidecar. No SDK required to build or test this library.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, BufReader, Read, Write};
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

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    State { state: Box<Snapshot> },
    Image { kind: String, jpeg: String },
    Error { error: String },
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
        let mut child = Command::new(&config.python)
            .arg("-u")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
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
    fn fake_sidecar_receives_config_and_is_reaped() {
        let directory = tempfile::tempdir().unwrap();
        let script = directory.path().join("fake.py");
        std::fs::write(&script, "import json,sys,time\nassert json.loads(sys.stdin.readline())['type']=='configure'\nprint(json.dumps({'type':'error','error':'fake SDK offline'}),flush=True)\ntime.sleep(60)\n").unwrap();
        let mut sidecar = Sidecar::spawn(&Config::default(), &script).unwrap();
        let pid = sidecar.child.id();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if let Some(event) = sidecar.poll().unwrap() {
                assert!(matches!(event, Event::Error { error } if error == "fake SDK offline"));
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
