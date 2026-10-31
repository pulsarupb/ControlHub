//! ControlHub state integration for the independent OAK-D library.
use crate::state::{AppState, Device, epoch};
use oakd::{Config, Event, Sidecar, Snapshot};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

pub fn run(state: AppState, script: std::path::PathBuf) {
    while !state.shutting_down.load(Ordering::SeqCst) {
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
        while !state.shutting_down.load(Ordering::SeqCst)
            && state.config.lock().unwrap().oak == *config
        {
            state.status.lock().unwrap().oak.update(Snapshot::default());
            std::thread::sleep(Duration::from_millis(100));
        }
        return Ok(());
    }
    let sidecar = Sidecar::spawn(config, script)?;
    let mut last_event = Instant::now();
    while !state.shutting_down.load(Ordering::SeqCst) && state.config.lock().unwrap().oak == *config
    {
        if let Some(event) = sidecar.recv_timeout(Duration::from_millis(100))? {
            last_event = Instant::now();
            match event {
                Event::State { cameras } => {
                    let mut cameras = *cameras;
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
                        let bytes = Event::image_bytes(&jpeg).map_err(|e| e.to_string())?;
                        state.oak_images.lock().unwrap()[slot][index] =
                            Some((bytes, Instant::now() - Duration::from_secs_f64(age)));
                    }
                }
                Event::Error { error } => return Err(error),
            }
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
        state.shutting_down.store(true, Ordering::SeqCst);
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
