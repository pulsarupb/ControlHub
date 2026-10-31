use crate::state::{AppState, DriveData};
use roboteq::{Controller, Telemetry};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

pub fn run(state: AppState) {
    loop {
        let config = state.config.lock().unwrap().drive.clone();
        state.stop_drive();
        {
            let mut status = state.status.lock().unwrap();
            status.drive = Default::default();
            status.drive.enabled = config.enabled;
            status.drive.simulated = state.simulate && config.enabled;
        }
        if !config.enabled {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        let result = session(&state, &config);
        state.stop_drive();
        if let Err(error) = result {
            state.status.lock().unwrap().drive.fail(error);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn session(state: &AppState, config: &roboteq::Config) -> Result<(), String> {
    let mut controllers = if state.simulate {
        None
    } else {
        Some([
            Controller::open(&config.ports[0]).map_err(|e| format!("controller A: {e}"))?,
            Controller::open(&config.ports[1]).map_err(|e| format!("controller B: {e}"))?,
        ])
    };
    let mut hardware_armed = false;
    let mut telemetry: [Telemetry; 2] = Default::default();
    let mut last_telemetry = Instant::now() - Duration::from_secs(1);
    let mut last_config_check = Instant::now();
    let mut config_check = 0;
    state
        .status
        .lock()
        .unwrap()
        .drive
        .update(DriveData::default());
    loop {
        if state.config.lock().unwrap().drive != *config {
            return Ok(());
        }
        let generation = state.drive_generation.load(Ordering::SeqCst);
        let now = Instant::now();
        let (armed, power) = {
            let mut drive = state.drive.lock().unwrap();
            (drive.armed(now), drive.output(config, now)?)
        };
        if let Some(c) = &mut controllers {
            // A failure drops BOTH controllers, whose Drop sends a best-effort
            // stop independently. Their hardware watchdogs remain the backstop.
            if armed && !hardware_armed {
                c[0].enable()
                    .map_err(|e| format!("controller A enable: {e}"))?;
                c[1].enable()
                    .map_err(|e| format!("controller B enable: {e}"))?;
            }
            if state.drive_generation.load(Ordering::SeqCst) != generation {
                // Stop may arrive while enable is awaiting ACKs. Drop both links
                // rather than forgetting that hardware was just released.
                return Ok(());
            }
            if armed {
                c[0].power(power[0])
                    .map_err(|e| format!("controller A power: {e}"))?;
                c[1].power(power[1])
                    .map_err(|e| format!("controller B power: {e}"))?;
            } else if hardware_armed {
                let a = c[0].stop();
                let b = c[1].stop();
                a.and(b).map_err(|e| e.to_string())?;
            }
            if last_telemetry.elapsed() >= Duration::from_millis(100) {
                telemetry = [
                    c[0].telemetry()
                        .map_err(|e| format!("controller A telemetry: {e}"))?,
                    c[1].telemetry()
                        .map_err(|e| format!("controller B telemetry: {e}"))?,
                ];
                for (index, t) in telemetry.iter().enumerate() {
                    let expected = if armed { 0 } else { 16 };
                    if t.faults != expected {
                        let name = ["A", "B"][index];
                        return Err(format!(
                            "controller {name} FF={} ({:#x}); expected {expected}; both stopped",
                            t.faults, t.faults
                        ));
                    }
                }
                last_telemetry = now;
            }
            if last_config_check.elapsed() >= Duration::from_millis(250) {
                for (index, controller) in c.iter_mut().enumerate() {
                    controller.verify_setting(config_check).map_err(|e| {
                        format!("controller {} configuration: {e}", ["A", "B"][index])
                    })?;
                }
                config_check = (config_check + 1) % 3;
                last_config_check = Instant::now();
            }
        }
        hardware_armed = armed;
        state.status.lock().unwrap().drive.update(DriveData {
            armed,
            power,
            controllers: telemetry.clone(),
        });
        std::thread::sleep(Duration::from_millis(20));
    }
}
