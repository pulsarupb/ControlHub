//! Encoder-free chassis yaw control. All angles/rates use radians and SI seconds.
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct YawConfig {
    pub enabled: bool,
    /// Positive steering drives left wheels faster: normally clockwise (-yaw).
    pub steering_yaw_sign: i8,
    pub max_rate_rad_s: f64,
    pub max_accel_rad_s2: f64,
    pub kp: f64,
    pub ki: f64,
    pub correction_limit: f64,
    pub filter_time_ms: u16,
    pub feedback_timeout_ms: u16,
    pub max_measured_rate_rad_s: f64,
    pub heading_hold: bool,
    pub heading_kp: f64,
    pub steering_deadband: f64,
}

impl Default for YawConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            steering_yaw_sign: -1,
            max_rate_rad_s: 0.6,
            max_accel_rad_s2: 0.6,
            kp: 0.15,
            ki: 0.03,
            correction_limit: 0.2,
            filter_time_ms: 100,
            feedback_timeout_ms: 250,
            max_measured_rate_rad_s: 3.0,
            heading_hold: true,
            heading_kp: 1.0,
            steering_deadband: 0.03,
        }
    }
}

impl YawConfig {
    pub fn validate(&self) -> Result<(), String> {
        if ![-1, 1].contains(&self.steering_yaw_sign)
            || ![
                (self.max_rate_rad_s, 0.01, 3.0),
                (self.max_accel_rad_s2, 0.01, 10.0),
                (self.kp, 0.0, 10.0),
                (self.ki, 0.0, 10.0),
                (self.correction_limit, 0.0, 0.5),
                (self.heading_kp, 0.0, 10.0),
                (self.steering_deadband, 0.0, 0.2),
                (self.max_measured_rate_rad_s, 0.1, 20.0),
            ]
            .iter()
            .all(|(v, lo, hi)| v.is_finite() && v >= lo && v <= hi)
            || !(50..=500).contains(&self.feedback_timeout_ms)
            || self.filter_time_ms > 1000
            || self.max_measured_rate_rad_s < self.max_rate_rad_s
        {
            return Err("invalid drive yaw_control limits/gains".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct YawFeedback {
    pub yaw: f64,
    /// Camera frame timestamp, not arrival time (seconds).
    pub timestamp: f64,
    pub received: Instant,
    pub epoch: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct YawStatus {
    pub active: bool,
    pub target_rate_rad_s: f64,
    pub measured_rate_rad_s: f64,
    pub correction: f64,
    pub heading_held: bool,
}

#[derive(Default)]
pub(crate) struct YawController {
    previous: Option<YawFeedback>,
    last_tick: Option<Instant>,
    integral: f64,
    heading: Option<f64>,
    ready: bool,
    pub status: YawStatus,
}

fn wrap(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

impl YawController {
    pub fn update(
        &mut self,
        c: &YawConfig,
        demand: (f32, f32),
        feedback: Option<YawFeedback>,
        now: Instant,
    ) -> Result<f32, String> {
        let f = feedback.ok_or("ZED yaw feedback unavailable; explicit enable required")?;
        let timeout = Duration::from_millis(c.feedback_timeout_ms.into());
        if !f.yaw.is_finite()
            || !f.timestamp.is_finite()
            || now < f.received
            || now.duration_since(f.received) >= timeout
        {
            return Err("ZED yaw feedback stale or invalid; explicit enable required".into());
        }
        if let Some(p) = self.previous {
            let dt = f.timestamp - p.timestamp;
            if f.epoch != p.epoch
                || dt < 0.0
                || dt > timeout.as_secs_f64()
                || (dt == 0.0 && (f.yaw != p.yaw || f.received != p.received))
            {
                return Err("ZED pose reset/discontinuity; explicit enable required".into());
            }
            if dt > 0.0 {
                let rate = wrap(f.yaw - p.yaw) / dt;
                if rate.abs() > c.max_measured_rate_rad_s {
                    return Err("ZED yaw jump; explicit enable required".into());
                }
                let alpha = if self.ready {
                    dt / (dt + f64::from(c.filter_time_ms) / 1000.0)
                } else {
                    1.0
                };
                self.status.measured_rate_rad_s += alpha * (rate - self.status.measured_rate_rad_s);
                self.ready = true;
            }
        }
        self.previous = Some(f);
        let dt = self.last_tick.replace(now).map_or(0.0, |p| {
            now.saturating_duration_since(p).as_secs_f64().min(0.1)
        });
        // Never rotate autonomously at a neutral joystick or while priming feedback.
        if !self.ready || (demand.0 == 0.0 && f64::from(demand.1).abs() <= c.steering_deadband) {
            self.integral = 0.0;
            self.heading = None;
            self.status.target_rate_rad_s = 0.0;
            self.status.correction = 0.0;
            self.status.active = false;
            self.status.heading_held = false;
            return Ok(0.0);
        }
        let sign = f64::from(c.steering_yaw_sign);
        let mut target = f64::from(demand.1) * sign * c.max_rate_rad_s;
        if c.heading_hold && f64::from(demand.1).abs() <= c.steering_deadband && demand.0 != 0.0 {
            let heading = *self.heading.get_or_insert(f.yaw);
            target =
                (c.heading_kp * wrap(heading - f.yaw)).clamp(-c.max_rate_rad_s, c.max_rate_rad_s);
        } else {
            self.heading = None;
        }
        self.status.heading_held = self.heading.is_some();
        let step = c.max_accel_rad_s2 * dt;
        let target = self.status.target_rate_rad_s
            + (target - self.status.target_rate_rad_s).clamp(-step, step);
        self.status.target_rate_rad_s = target;
        let error = (target - self.status.measured_rate_rad_s) * sign;
        let ff = target / (sign * c.max_rate_rad_s);
        let candidate =
            (self.integral + c.ki * error * dt).clamp(-c.correction_limit, c.correction_limit);
        let raw = c.kp * error + candidate;
        let output = ff + raw.clamp(-c.correction_limit, c.correction_limit);
        // Conditional integration includes correction, steering and wheel mixer saturation.
        let correction_blocked = raw.abs() > c.correction_limit && error * raw > 0.0;
        let mixer_blocked = output.abs() + f64::from(demand.0).abs() > 1.0 && error * output > 0.0;
        if !correction_blocked && !mixer_blocked {
            self.integral = candidate;
        }
        self.status.correction =
            (c.kp * error + self.integral).clamp(-c.correction_limit, c.correction_limit);
        self.status.active = true;
        Ok((ff + self.status.correction).clamp(-1.0, 1.0) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(t: Instant, stamp: f64, yaw: f64) -> YawFeedback {
        YawFeedback {
            yaw,
            timestamp: stamp,
            received: t,
            epoch: 0,
        }
    }
    #[test]
    fn sign_wrap_and_heading_hold() {
        let c = YawConfig::default();
        let mut pi = YawController::default();
        let t = Instant::now();
        let edge = std::f64::consts::PI - 0.001;
        pi.update(&c, (0.3, 0.0), Some(sample(t, 1.0, edge)), t)
            .unwrap();
        let t = t + Duration::from_millis(20);
        pi.update(&c, (0.3, 0.0), Some(sample(t, 1.02, -edge)), t)
            .unwrap();
        assert!(pi.status.measured_rate_rad_s.abs() < 0.2);
        assert!(pi.status.heading_held);
        assert!(pi.status.correction > 0.0); // positive yaw drift -> clockwise correction
    }
    #[test]
    fn stale_reset_jump_and_missing_fail_closed() {
        let c = YawConfig::default();
        let t = Instant::now();
        for mode in 0..4 {
            let mut pi = YawController::default();
            let f = sample(t, 1.0, 0.0);
            pi.update(&c, (0.0, 0.5), Some(f), t).unwrap();
            let mut next = sample(t + Duration::from_millis(20), 1.02, 0.0);
            if mode == 0 {
                next.epoch = 1;
            }
            if mode == 1 {
                next.yaw = 1.0;
            }
            let now = if mode == 2 {
                t + Duration::from_secs(1)
            } else {
                next.received
            };
            assert!(
                pi.update(
                    &c,
                    (0.0, 0.5),
                    if mode == 3 { None } else { Some(next) },
                    now
                )
                .is_err()
            );
        }
    }
    #[test]
    fn neutral_and_saturation_do_not_wind_up() {
        let c = YawConfig::default();
        let mut pi = YawController::default();
        let start = Instant::now();
        for i in 0..500 {
            let t = start + Duration::from_millis(i * 20);
            pi.update(
                &c,
                (1.0, 1.0),
                Some(sample(t, 1.0 + i as f64 * 0.02, 0.0)),
                t,
            )
            .unwrap();
        }
        assert_eq!(pi.integral, 0.0);
        let t = start + Duration::from_secs(10);
        assert_eq!(
            pi.update(&c, (0.0, 0.0), Some(sample(t, 11.0, 0.0)), t)
                .unwrap(),
            0.0
        );
        assert_eq!(pi.integral, 0.0);
    }

    #[test]
    fn pi_compensates_reduced_turn_response() {
        let c = YawConfig {
            ki: 0.3,
            ..Default::default()
        };
        let mut pi = YawController::default();
        let start = Instant::now();
        let mut yaw = 0.0;
        for i in 0..2000 {
            let t = start + Duration::from_millis(i * 20);
            let steering = pi
                .update(
                    &c,
                    (0.2, 0.3),
                    Some(sample(t, 1.0 + i as f64 * 0.02, yaw)),
                    t,
                )
                .unwrap();
            // A 25% loss of turning response, with no wheel encoders.
            yaw += -f64::from(steering) * c.max_rate_rad_s * 0.75 * 0.02;
        }
        assert!((pi.status.measured_rate_rad_s + 0.18).abs() < 0.005);
        assert!(pi.status.correction > 0.08);
    }

    #[test]
    fn repeated_frames_do_not_integrate_twice_or_mask_frozen_camera() {
        let c = YawConfig::default();
        let start = Instant::now();
        let mut pi = YawController::default();
        let f = sample(start, 1.0, 0.0);
        pi.update(&c, (0.0, 0.5), Some(f), start).unwrap();
        pi.update(&c, (0.0, 0.5), Some(f), start + Duration::from_millis(10))
            .unwrap();
        assert!(!pi.ready);
        let frozen = YawFeedback {
            received: start + Duration::from_millis(20),
            ..f
        };
        assert!(
            pi.update(&c, (0.0, 0.5), Some(frozen), frozen.received)
                .is_err()
        );
    }

    #[test]
    fn invalid_gains_and_limits_are_rejected() {
        for c in [
            YawConfig {
                kp: f64::NAN,
                ..Default::default()
            },
            YawConfig {
                steering_yaw_sign: 0,
                ..Default::default()
            },
            YawConfig {
                feedback_timeout_ms: 0,
                ..Default::default()
            },
            YawConfig {
                correction_limit: 1.0,
                ..Default::default()
            },
        ] {
            assert!(c.validate().is_err());
        }
    }

    #[test]
    fn heading_error_corrects_reverse_travel_and_manual_steering_releases_hold() {
        let c = YawConfig::default();
        let start = Instant::now();
        let mut pi = YawController::default();
        for i in 0..2 {
            let t = start + Duration::from_millis(i * 20);
            pi.update(
                &c,
                (-0.3, 0.0),
                Some(sample(t, 1.0 + i as f64 * 0.02, 0.0)),
                t,
            )
            .unwrap();
        }
        let t = start + Duration::from_millis(40);
        pi.update(&c, (-0.3, 0.0), Some(sample(t, 1.04, 0.01)), t)
            .unwrap();
        assert!(pi.status.heading_held);
        assert!(pi.status.target_rate_rad_s < 0.0);
        let t = start + Duration::from_millis(60);
        pi.update(&c, (-0.3, 0.3), Some(sample(t, 1.06, 0.01)), t)
            .unwrap();
        assert!(!pi.status.heading_held);
        let t = start + Duration::from_millis(80);
        assert_eq!(
            pi.update(&c, (0.0, 0.0), Some(sample(t, 1.08, 0.01)), t)
                .unwrap(),
            0.0
        );
        assert!(!pi.status.active);
    }
}
