use crate::state::{AppState, RtkData};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn rtk_worker(state: AppState) {
    loop {
        let config = state.config.lock().unwrap().rtk.clone();
        {
            let mut s = state.status.lock().unwrap();
            s.rtk = Default::default();
            s.rtk.enabled = config.enabled;
            s.rtk.simulated = state.simulate && config.enabled;
        }
        if !config.enabled {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        let result = rtk_session(&state, &config);
        if let Err(error) = result {
            state.status.lock().unwrap().rtk.fail(error);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn rtk_session(state: &AppState, config: &rtk::Config) -> Result<(), String> {
    if state.simulate {
        while state.config.lock().unwrap().rtk == *config {
            // No fabricated fix or RTK quality: simulation labels only connectivity.
            state.status.lock().unwrap().rtk.update(RtkData::default());
            std::thread::sleep(Duration::from_millis(100));
        }
        return Ok(());
    }
    let mut receiver = rtk::Receiver::open(config).map_err(|e| e.to_string())?;
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(16);
    let stopped = Arc::new(AtomicBool::new(false));
    let ntrip_state = Arc::new(Mutex::new((false, None::<String>)));
    let network = if config.ntrip.enabled {
        let cfg = config.ntrip.clone();
        let stop = stopped.clone();
        let network_state = ntrip_state.clone();
        Some(std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let result = (|| -> std::io::Result<()> {
                    let mut client = rtk::NtripClient::connect(&cfg)?;
                    *network_state.lock().unwrap() = (true, None);
                    while !stop.load(Ordering::SeqCst) {
                        let bytes = client.read()?;
                        if !bytes.is_empty() && tx.try_send(bytes).is_err() {
                            return Err(std::io::Error::other(
                                "correction queue full or receiver stopped",
                            ));
                        }
                    }
                    Ok(())
                })();
                *network_state.lock().unwrap() = (false, result.err().map(|e| e.to_string()));
                for _ in 0..30 {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }))
    } else {
        None
    };
    let mut data = RtkData::default();
    let result = (|| -> std::io::Result<()> {
        while state.config.lock().unwrap().rtk == *config {
            for bytes in rx.try_iter() {
                receiver.corrections(&bytes)?;
                data.correction_bytes += bytes.len() as u64;
            }
            for fix in receiver.poll()? {
                data.fix = Some(fix);
            }
            data.bytes_read = receiver.bytes_read;
            data.parse_errors = receiver.parse_errors;
            let (connected, error) = ntrip_state.lock().unwrap().clone();
            data.ntrip_connected = connected;
            data.ntrip_error = error;
            state.status.lock().unwrap().rtk.update(data.clone());
        }
        Ok(())
    })();
    stopped.store(true, Ordering::SeqCst);
    if let Some(thread) = network {
        let _ = thread.join();
    }
    result.map_err(|e| e.to_string())
}

pub fn zed_worker(state: AppState, script: std::path::PathBuf) {
    loop {
        let config = state.config.lock().unwrap().zed.clone();
        *state.yaw_feedback.lock().unwrap() = None;
        {
            let mut s = state.status.lock().unwrap();
            s.zed = Default::default();
            s.zed.enabled = config.enabled;
            s.zed.simulated = state.simulate && config.enabled;
        }
        *state.images.lock().unwrap() = Default::default();
        state.zed_reset.store(false, Ordering::SeqCst);
        state.zed_generate_mesh.store(false, Ordering::SeqCst);
        *state.zed_mesh.lock().unwrap() = Default::default();
        if !config.enabled {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        let result = zed_session(&state, &config, &script);
        *state.yaw_feedback.lock().unwrap() = None;
        if state.config.lock().unwrap().drive.yaw_control.enabled {
            state.stop_drive_with("ZED session ended");
        }
        if let Err(error) = result {
            let mut mesh = state.zed_mesh.lock().unwrap();
            mesh.generating = false;
            mesh.error = Some(error.clone());
            state.status.lock().unwrap().zed.fail(error);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn zed_session(
    state: &AppState,
    config: &zed::Config,
    script: &std::path::Path,
) -> Result<(), String> {
    if state.simulate {
        while state.config.lock().unwrap().zed == *config {
            state.status.lock().unwrap().zed.update(zed::Snapshot {
                tracking_state: "SIMULATED — no measured pose".into(),
                ..Default::default()
            });
            std::thread::sleep(Duration::from_millis(100));
        }
        return Ok(());
    }
    let mut sidecar = zed::Sidecar::spawn(config, script).map_err(|e| e.to_string())?;
    let mut last_event = Instant::now();
    while state.config.lock().unwrap().zed == *config {
        if state.zed_reset.swap(false, Ordering::SeqCst) {
            *state.zed_mesh.lock().unwrap() = Default::default();
            *state.yaw_feedback.lock().unwrap() = None;
            sidecar
                .send(&serde_json::json!({"type": "reset"}))
                .map_err(|e| e.to_string())?;
        }
        if state.zed_generate_mesh.swap(false, Ordering::SeqCst) {
            sidecar
                .send(&serde_json::json!({"type": "generate_mesh"}))
                .map_err(|e| e.to_string())?;
        }
        while let Some(event) = sidecar.poll().map_err(|e| e.to_string())? {
            last_event = Instant::now();
            match event {
                zed::Event::Mesh { mesh, error } => {
                    let mut snapshot = state.zed_mesh.lock().unwrap();
                    snapshot.generating = false;
                    snapshot.error = error;
                    if mesh.is_some() {
                        snapshot.mesh = mesh;
                        snapshot.generated_at = Some(crate::state::epoch());
                    }
                }
                zed::Event::Odometry {
                    orientation,
                    timestamp,
                    captured_at,
                    epoch,
                } => {
                    let age = crate::state::epoch() - captured_at;
                    let yaw = orientation.and_then(|q| config.base_yaw(q));
                    let feedback = yaw
                        .filter(|_| {
                            timestamp.is_finite()
                                && timestamp > 0.0
                                && age.is_finite()
                                && (0.0..0.5).contains(&age)
                        })
                        .map(|yaw| roboteq::YawFeedback {
                            yaw,
                            timestamp,
                            epoch,
                            received: Instant::now() - Duration::from_secs_f64(age),
                        });
                    *state.yaw_feedback.lock().unwrap() = feedback;
                    if feedback.is_none() && state.config.lock().unwrap().drive.yaw_control.enabled
                    {
                        state.stop_drive_with("ZED yaw feedback lost");
                    }
                }
                zed::Event::State { state: data } => state.status.lock().unwrap().zed.update(*data),
                zed::Event::Image { kind, jpeg } => {
                    let bytes = zed::Event::image_bytes(&jpeg).map_err(|e| e.to_string())?;
                    let index = match kind.as_str() {
                        "color" => 0,
                        "depth" => 1,
                        _ => return Err("invalid ZED image kind".into()),
                    };
                    state.images.lock().unwrap()[index] = bytes;
                }
                zed::Event::Error { error } => return Err(error),
            }
        }
        let timeout = if state.zed_mesh.lock().unwrap().generating {
            180
        } else {
            30
        };
        if last_event.elapsed() > Duration::from_secs(timeout) {
            return Err("ZED sidecar stalled".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}
