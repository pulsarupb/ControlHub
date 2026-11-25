use crate::config::Config;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

pub fn epoch() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Why a motion device last stopped. Persists until the next explicit enable.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct StopInfo {
    pub reason: String,
    pub at: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct StopEvent {
    pub devices: Vec<&'static str>,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DriveSummary {
    pub enabled: bool,
    pub connected: bool,
    pub simulated: bool,
    pub updated_at: Option<f64>,
    pub error: Option<String>,
    pub stop_reason: Option<StopInfo>,
    pub armed: bool,
    pub power: [[i16; 2]; 2],
    pub faults: [i32; 2],
    pub controllers: [roboteq::Telemetry; 2],
    pub yaw_control: roboteq::YawStatus,
}

#[derive(Clone, Debug, Serialize)]
pub struct ZedSummary {
    pub camera_ready: bool,
    pub camera_model: String,
    pub serial: String,
    pub fps: f64,
    pub pose: Option<zed::Pose>,
    pub imu: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeviceWithStop<T: Serialize> {
    pub enabled: bool,
    pub connected: bool,
    pub simulated: bool,
    pub updated_at: Option<f64>,
    pub error: Option<String>,
    pub data: T,
    pub stop_reason: Option<StopInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DevicesSummary {
    pub time: f64,
    pub simulate: bool,
    pub kinova: DeviceWithStop<ArmData>,
    pub zed: Device<ZedSummary>,
    pub rtk: Device<RtkData>,
    pub oak: Device<oakd::Snapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FullStatusSummary {
    pub time: f64,
    pub simulate: bool,
    pub devices: FullStatusDevices,
}

#[derive(Clone, Debug, Serialize)]
pub struct FullStatusDevices {
    pub drive: DeviceWithStop<DriveData>,
    pub kinova: DeviceWithStop<ArmData>,
    pub rtk: Device<RtkData>,
    pub zed: Device<ZedSummary>,
    pub oak: Device<oakd::Snapshot>,
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
    pub drive_stop_info: Arc<Mutex<Option<StopInfo>>>,
    pub arm_stop_info: Arc<Mutex<Option<StopInfo>>>,
    pub ws_owner: Arc<Mutex<Option<u64>>>,
    pub ws_next_id: Arc<AtomicU64>,
    pub stop_events: broadcast::Sender<StopEvent>,
    pub zed_reset: Arc<AtomicBool>,
    pub zed_generate_mesh: Arc<AtomicBool>,
    pub zed_mesh: Arc<Mutex<zed::MeshSnapshot>>,
    pub yaw_feedback: Arc<Mutex<Option<roboteq::YawFeedback>>>,
    pub images: Arc<Mutex<[Vec<u8>; 2]>>,
    pub oak_images: Arc<Mutex<oakd::Images>>,
    pub d435: d435::Camera,
    pub simulate: bool,
    pub shutting_down: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(
        config: Config,
        path: String,
        simulate: bool,
    ) -> (Self, tokio::sync::mpsc::Receiver<crate::arm::Request>) {
        let (arm_tx, arm_rx) = tokio::sync::mpsc::channel(8);
        let (stop_events, _) = broadcast::channel(32);
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
                drive_stop_info: Default::default(),
                arm_stop_info: Default::default(),
                ws_owner: Arc::new(Mutex::new(None)),
                ws_next_id: Arc::new(AtomicU64::new(1)),
                stop_events,
                zed_reset: Arc::new(AtomicBool::new(false)),
                zed_generate_mesh: Arc::new(AtomicBool::new(false)),
                zed_mesh: Default::default(),
                yaw_feedback: Default::default(),
                images: Default::default(),
                oak_images: Default::default(),
                d435: Default::default(),
                simulate,
                shutting_down: Arc::new(AtomicBool::new(false)),
            },
            arm_rx,
        )
    }
    pub fn record_drive_stop(&self, reason: impl Into<String>) {
        let reason = reason.into();
        eprintln!("drive stopped: {reason}");
        *self.drive_stop_info.lock().unwrap() = Some(StopInfo {
            reason: reason.clone(),
            at: epoch(),
        });
        let _ = self.stop_events.send(StopEvent {
            devices: vec!["drive"],
            reason,
        });
    }
    pub fn record_arm_stop(&self, reason: impl Into<String>) {
        let reason = reason.into();
        eprintln!("arm stopped: {reason}");
        *self.arm_stop_info.lock().unwrap() = Some(StopInfo {
            reason: reason.clone(),
            at: epoch(),
        });
        let _ = self.stop_events.send(StopEvent {
            devices: vec!["kinova"],
            reason,
        });
    }
    pub fn clear_drive_stop(&self) {
        *self.drive_stop_info.lock().unwrap() = None;
    }
    pub fn clear_arm_stop(&self) {
        *self.arm_stop_info.lock().unwrap() = None;
    }
    #[allow(dead_code)]
    pub fn stop_drive(&self) {
        self.stop_drive_with("stopped");
    }
    pub fn stop_drive_with(&self, reason: impl Into<String>) {
        let was_armed = {
            let mut drive = self.drive.lock().unwrap();
            let was_armed = drive.is_armed();
            drive.stop();
            was_armed
        };
        self.drive_generation.fetch_add(1, Ordering::SeqCst);
        if was_armed {
            self.record_drive_stop(reason);
        }
    }
    /// Stop without replacing an already-recorded, more specific reason.
    pub fn stop_drive_preserving_reason(&self, reason: impl Into<String>) {
        let (was_armed, internal_reason) = {
            let mut drive = self.drive.lock().unwrap();
            let was_armed = drive.is_armed();
            let internal_reason = drive.take_stop_reason();
            drive.stop();
            (was_armed, internal_reason)
        };
        self.drive_generation.fetch_add(1, Ordering::SeqCst);
        if was_armed {
            let mut stop = self.drive_stop_info.lock().unwrap();
            if stop.is_none() {
                let reason = internal_reason.unwrap_or_else(|| reason.into());
                eprintln!("drive stopped: {reason}");
                *stop = Some(StopInfo {
                    reason: reason.clone(),
                    at: epoch(),
                });
                let _ = self.stop_events.send(StopEvent {
                    devices: vec!["drive"],
                    reason,
                });
            }
        }
    }
    #[allow(dead_code)]
    pub fn stop_arm(&self) {
        self.stop_arm_with("stopped");
    }
    pub fn stop_arm_with(&self, reason: impl Into<String>) {
        let was_armed = self.arm_enabled.load(Ordering::SeqCst);
        {
            let mut jog = self.arm_jog.lock().unwrap();
            self.arm_enabled.store(false, Ordering::SeqCst);
            self.arm_stop.fetch_add(1, Ordering::SeqCst);
            *jog = None;
        }
        if was_armed {
            self.record_arm_stop(reason);
        }
    }
    /// Stop without replacing an already-recorded, more specific reason.
    pub fn stop_arm_preserving_reason(&self, reason: impl Into<String>) {
        let was_armed = self.arm_enabled.load(Ordering::SeqCst);
        {
            let mut jog = self.arm_jog.lock().unwrap();
            self.arm_enabled.store(false, Ordering::SeqCst);
            self.arm_stop.fetch_add(1, Ordering::SeqCst);
            *jog = None;
        }
        if was_armed {
            let mut stop = self.arm_stop_info.lock().unwrap();
            if stop.is_none() {
                let reason = reason.into();
                eprintln!("arm stopped: {reason}");
                *stop = Some(StopInfo {
                    reason: reason.clone(),
                    at: epoch(),
                });
                let _ = self.stop_events.send(StopEvent {
                    devices: vec!["kinova"],
                    reason,
                });
            }
        }
    }
    #[allow(dead_code)]
    pub fn stop_all(&self) {
        self.stop_all_with("operator stop");
    }
    pub fn stop_all_with(&self, reason: impl Into<String>) {
        let reason = reason.into();
        let drive_was_armed = {
            let mut drive = self.drive.lock().unwrap();
            let was_armed = drive.is_armed();
            drive.stop();
            was_armed
        };
        self.drive_generation.fetch_add(1, Ordering::SeqCst);

        let arm_was_armed = self.arm_enabled.load(Ordering::SeqCst);
        {
            let mut jog = self.arm_jog.lock().unwrap();
            self.arm_enabled.store(false, Ordering::SeqCst);
            self.arm_stop.fetch_add(1, Ordering::SeqCst);
            *jog = None;
        }

        let mut stopped = Vec::new();
        let at = epoch();
        if drive_was_armed {
            eprintln!("drive stopped: {reason}");
            *self.drive_stop_info.lock().unwrap() = Some(StopInfo {
                reason: reason.clone(),
                at,
            });
            stopped.push("drive");
        }
        if arm_was_armed {
            eprintln!("arm stopped: {reason}");
            *self.arm_stop_info.lock().unwrap() = Some(StopInfo {
                reason: reason.clone(),
                at,
            });
            stopped.push("kinova");
        }
        if !stopped.is_empty() {
            let _ = self.stop_events.send(StopEvent {
                devices: stopped,
                reason,
            });
        }
    }

    /// Read-only drive summary for WebSocket push. Never touches `Drive::armed(now)`
    /// or `take_stop_reason()`, keeping `motor::run` and explicit control/stop paths
    /// as the only latch writers.
    pub fn drive_summary(&self) -> DriveSummary {
        let armed = self.drive.lock().unwrap().is_armed();
        let stop_reason = self.drive_stop_info.lock().unwrap().clone();
        let status = self.status.lock().unwrap();
        let drive = &status.drive;
        DriveSummary {
            enabled: drive.enabled,
            connected: drive.connected,
            simulated: drive.simulated,
            updated_at: drive.updated_at,
            error: drive.error.clone(),
            stop_reason,
            armed,
            power: drive.data.power,
            faults: [
                drive.data.controllers[0].faults,
                drive.data.controllers[1].faults,
            ],
            controllers: drive.data.controllers.clone(),
            yaw_control: drive.data.yaw_control.clone(),
        }
    }

    /// Read-only device summary (without `zed.path/points/colors`) built under the lock.
    pub fn devices_summary(&self) -> DevicesSummary {
        let arm_armed = self.arm_enabled.load(Ordering::SeqCst);
        let arm_stop = self.arm_stop_info.lock().unwrap().clone();
        let status = self.status.lock().unwrap();
        let mut kinova_data = status.kinova.data.clone();
        kinova_data.armed = arm_armed;
        DevicesSummary {
            time: epoch(),
            simulate: self.simulate,
            kinova: DeviceWithStop {
                enabled: status.kinova.enabled,
                connected: status.kinova.connected,
                simulated: status.kinova.simulated,
                updated_at: status.kinova.updated_at,
                error: status.kinova.error.clone(),
                data: kinova_data,
                stop_reason: arm_stop,
            },
            zed: Device {
                enabled: status.zed.enabled,
                connected: status.zed.connected,
                simulated: status.zed.simulated,
                updated_at: status.zed.updated_at,
                error: status.zed.error.clone(),
                data: ZedSummary {
                    camera_ready: status.zed.data.camera_ready,
                    camera_model: status.zed.data.camera_model.clone(),
                    serial: status.zed.data.serial.clone(),
                    fps: status.zed.data.fps,
                    pose: status.zed.data.pose.clone(),
                    imu: status.zed.data.imu.clone(),
                    error: status.zed.data.error.clone(),
                },
            },
            rtk: status.rtk.clone(),
            oak: status.oak.clone(),
        }
    }

    /// Lean `/api/status` DTO built inside the lock without cloning ZED point-cloud/path arrays.
    pub fn full_status_summary(&self) -> FullStatusSummary {
        let (armed, stop_reason) = {
            let mut drive = self.drive.lock().unwrap();
            let armed = drive.armed(Instant::now());
            (armed, drive.take_stop_reason())
        };
        if let Some(reason) = stop_reason {
            self.record_drive_stop(reason);
        }
        let arm_armed = self.arm_enabled.load(Ordering::SeqCst);
        let drive_stop = self.drive_stop_info.lock().unwrap().clone();
        let arm_stop = self.arm_stop_info.lock().unwrap().clone();
        let status = self.status.lock().unwrap();
        let mut drive_data = status.drive.data.clone();
        drive_data.armed = armed;
        let mut kinova_data = status.kinova.data.clone();
        kinova_data.armed = arm_armed;
        FullStatusSummary {
            time: epoch(),
            simulate: self.simulate,
            devices: FullStatusDevices {
                drive: DeviceWithStop {
                    enabled: status.drive.enabled,
                    connected: status.drive.connected,
                    simulated: status.drive.simulated,
                    updated_at: status.drive.updated_at,
                    error: status.drive.error.clone(),
                    data: drive_data,
                    stop_reason: drive_stop,
                },
                kinova: DeviceWithStop {
                    enabled: status.kinova.enabled,
                    connected: status.kinova.connected,
                    simulated: status.kinova.simulated,
                    updated_at: status.kinova.updated_at,
                    error: status.kinova.error.clone(),
                    data: kinova_data,
                    stop_reason: arm_stop,
                },
                rtk: status.rtk.clone(),
                zed: Device {
                    enabled: status.zed.enabled,
                    connected: status.zed.connected,
                    simulated: status.zed.simulated,
                    updated_at: status.zed.updated_at,
                    error: status.zed.error.clone(),
                    data: ZedSummary {
                        camera_ready: status.zed.data.camera_ready,
                        camera_model: status.zed.data.camera_model.clone(),
                        serial: status.zed.data.serial.clone(),
                        fps: status.zed.data.fps,
                        pose: status.zed.data.pose.clone(),
                        imu: status.zed.data.imu.clone(),
                        error: status.zed.data.error.clone(),
                    },
                },
                oak: status.oak.clone(),
            },
        }
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
    pub yaw_control: roboteq::YawStatus,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ArmData {
    pub armed: bool,
    pub joints: Vec<f32>,
    pub pose: Option<[f32; 6]>,
    pub gripper: Option<f32>,
    pub arm_state: Option<i32>,
    pub preview: Option<crate::arm::Preview>,
    pub rear_boundary: Option<crate::arm::RearBoundary>,
    pub rear_boundary_error: Option<String>,
    pub telemetry_error: Option<String>,
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
    pub oak: Device<oakd::Snapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_stop_reason_is_recorded_only_while_armed() {
        let (state, _) = AppState::new(Default::default(), String::new(), true);
        state.stop_drive_with("operator stop");
        assert!(state.drive_stop_info.lock().unwrap().is_none());

        state.drive.lock().unwrap().enable(Instant::now());
        state.stop_drive_with("operator stop");
        assert_eq!(
            state
                .drive_stop_info
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .reason,
            "operator stop"
        );

        state.drive.lock().unwrap().enable(Instant::now());
        state.record_drive_stop("first");
        state.stop_drive_preserving_reason("second");
        assert_eq!(
            state
                .drive_stop_info
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .reason,
            "first"
        );

        state.clear_drive_stop();
        assert!(state.drive_stop_info.lock().unwrap().is_none());
    }

    #[test]
    fn arm_stop_reason_is_recorded_only_while_armed() {
        let (state, _) = AppState::new(Default::default(), String::new(), true);
        state.stop_arm_with("operator stop");
        assert!(state.arm_stop_info.lock().unwrap().is_none());

        state.arm_enabled.store(true, Ordering::SeqCst);
        state.stop_arm_with("arm heartbeat expired (500 ms)");
        assert_eq!(
            state.arm_stop_info.lock().unwrap().as_ref().unwrap().reason,
            "arm heartbeat expired (500 ms)"
        );

        state.clear_arm_stop();
        assert!(state.arm_stop_info.lock().unwrap().is_none());
    }
}
