//! Standalone D435 color capture. No VLM, inference, HTTP service, or depth SDK.
mod capture;
mod detect;

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const MAX_AGE: Duration = Duration::from_secs(3);

#[derive(Default)]
struct State {
    frame: Option<(Vec<u8>, Instant)>,
    error: Option<String>,
}

#[derive(Clone, Default)]
pub struct Camera {
    state: Arc<Mutex<State>>,
}

impl Camera {
    /// Run on one dedicated thread; missing, busy or disconnected cameras are retried.
    pub fn run(&self, shutdown: &AtomicBool) {
        while !shutdown.load(Ordering::SeqCst) {
            let result = self.capture(shutdown);
            let mut state = self.state.lock().unwrap();
            state.frame = None;
            state.error = result.err();
            drop(state);
            for _ in 0..20 {
                if shutdown.load(Ordering::SeqCst) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    fn capture(&self, shutdown: &AtomicBool) -> Result<(), String> {
        let device = detect::detect().ok_or("D435 color camera not detected")?;
        let mut camera = capture::open(&device)
            .map_err(|e| format!("Cannot open D435 (camera may be in use): {e}"))?;
        while !shutdown.load(Ordering::SeqCst) {
            let jpg = capture::frame(&mut camera)?;
            let mut state = self.state.lock().unwrap();
            state.frame = Some((jpg, Instant::now()));
            state.error = None;
        }
        Ok(())
    }

    pub fn snapshot(&self, crosshair: bool) -> Result<Vec<u8>, String> {
        let jpg = {
            let state = self.state.lock().unwrap();
            let (jpg, captured) = state.frame.as_ref().ok_or_else(|| {
                state
                    .error
                    .clone()
                    .unwrap_or_else(|| "Waiting for D435 camera".into())
            })?;
            if captured.elapsed() >= MAX_AGE {
                return Err("D435 camera frame stale".into());
            }
            jpg.clone()
        };
        if crosshair {
            capture::crosshair(&jpg)
        } else {
            Ok(jpg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_and_stale_frames_are_not_shown() {
        let camera = Camera::default();
        assert!(camera.snapshot(false).is_err());
        camera.state.lock().unwrap().frame = Some((vec![1], Instant::now() - MAX_AGE));
        assert!(camera.snapshot(false).unwrap_err().contains("stale"));
    }

    #[test]
    fn jpeg_encoding_and_crosshair_work_without_hardware() {
        let camera = Camera::default();
        let jpg = capture::encode(&vec![0; 80 * 80 * 3], 80, 80).unwrap();
        camera.state.lock().unwrap().frame = Some((jpg.clone(), Instant::now()));
        assert_eq!(camera.snapshot(false).unwrap(), jpg);
        let marked = camera.snapshot(true).unwrap();
        let rgb = image::load_from_memory(&marked).unwrap().to_rgb8();
        assert_eq!(rgb.dimensions(), (80, 80));
        assert!(rgb.get_pixel(40, 40)[0] > 100);
        assert_ne!(marked, jpg);
    }

    #[test]
    fn shutdown_does_not_open_devices() {
        Camera::default().run(&AtomicBool::new(true));
    }
}
