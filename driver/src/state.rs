use crate::config::Config;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub fn epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub config_path: Arc<String>,
    pub status: Arc<Mutex<Status>>,
    pub drive: Arc<Mutex<roboteq::Drive>>,
    pub drive_generation: Arc<AtomicU64>,
    pub arm_enabled: Arc<AtomicBool>,
    pub arm_stop: Arc<AtomicU64>,
    pub arm_jog: Arc<Mutex<Option<(crate::arm::Jog, Instant)>>>,
    pub arm_seen: Arc<Mutex<Instant>>,
    pub arm_tx: tokio::sync::mpsc::Sender<crate::arm::Request>,
    pub zed_reset: Arc<AtomicBool>,
    pub images: Arc<Mutex<[Vec<u8>; 2]>>,
    pub simulate: bool,
}

impl AppState {
    pub fn new(
        config: Config,
        path: String,
        simulate: bool,
    ) -> (Self, tokio::sync::mpsc::Receiver<crate::arm::Request>) {
        let (arm_tx, arm_rx) = tokio::sync::mpsc::channel(8);
        (
            Self {
                config: Arc::new(Mutex::new(config)),
                config_path: Arc::new(path),
                status: Default::default(),
                drive: Default::default(),
                drive_generation: Arc::new(AtomicU64::new(0)),
                arm_enabled: Arc::new(AtomicBool::new(false)),
                arm_stop: Arc::new(AtomicU64::new(0)),
                arm_jog: Default::default(),
                arm_seen: Arc::new(Mutex::new(Instant::now())),
                arm_tx,
                zed_reset: Arc::new(AtomicBool::new(false)),
                images: Default::default(),
                simulate,
            },
            arm_rx,
        )
    }
    pub fn stop_drive(&self) {
        self.drive.lock().unwrap().stop();
        self.drive_generation.fetch_add(1, Ordering::SeqCst);
    }
    pub fn stop_arm(&self) {
        let mut jog = self.arm_jog.lock().unwrap();
        self.arm_enabled.store(false, Ordering::SeqCst);
        self.arm_stop.fetch_add(1, Ordering::SeqCst);
        *jog = None;
    }
    pub fn stop_all(&self) {
        self.stop_drive();
        self.stop_arm();
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Device<T: Serialize> {
    pub enabled: bool,
    pub connected: bool,
    pub simulated: bool,
    pub updated_at: Option<f64>,
    pub error: Option<String>,
    pub data: T,
}

impl<T: Serialize + Default> Default for Device<T> {
    fn default() -> Self {
        Self {
            enabled: false,
            connected: false,
            simulated: false,
            updated_at: None,
            error: None,
            data: T::default(),
        }
    }
}

impl<T: Serialize> Device<T> {
    pub fn update(&mut self, data: T) {
        self.data = data;
        self.connected = true;
        self.updated_at = Some(epoch());
        self.error = None;
    }
    pub fn fail(&mut self, error: impl ToString) {
        self.connected = false;
        self.error = Some(error.to_string());
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DriveData {
    pub armed: bool,
    pub power: [[i16; 2]; 2],
    pub controllers: [roboteq::Telemetry; 2],
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ArmData {
    pub armed: bool,
    pub joints: Vec<f32>,
    pub pose: Option<[f32; 6]>,
    pub gripper: Option<f32>,
    pub arm_state: Option<i32>,
    pub preview: Option<crate::arm::Preview>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RtkData {
    pub fix: Option<rtk::Fix>,
    pub bytes_read: u64,
    pub parse_errors: u64,
    pub ntrip_connected: bool,
    pub correction_bytes: u64,
    pub ntrip_error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Status {
    pub drive: Device<DriveData>,
    pub kinova: Device<ArmData>,
    pub rtk: Device<RtkData>,
    pub zed: Device<zed::Snapshot>,
}
