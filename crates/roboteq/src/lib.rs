//! Minimal synchronous RoboteQ SDC2130 control. No chassis, web or ROS dependency.
mod protocol;
mod yaw;
pub use protocol::Controller;
pub use yaw::{YawConfig, YawFeedback, YawStatus};

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const WATCHDOG_MS: u64 = 500;
pub const DEFAULT_RAMP_TIME_MS: u16 = 250;
pub const MAX_RAMP_TIME_MS: u16 = 5000;
const LEGACY_MAX_RAMP_TIME_SECONDS: f64 = 0.25;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Motor {
    pub controller: usize,
    pub channel: u8,
    pub direction: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub ports: [String; 2],
    /// Order: left front, right front, left back, right back.
    pub motors: [Motor; 4],
    /// RoboteQ power units, not RPM; full scale is 1000.
    pub power_limit: i16,
    /// Milliseconds to ramp from zero to full power. Zero disables the ramp.
    #[serde(
        default = "default_ramp_time_ms",
        alias = "acceleration_time",
        deserialize_with = "deserialize_ramp_time_ms"
    )]
    pub acceleration_time_ms: u16,
    /// Milliseconds to ramp from full power to zero. Zero disables the ramp.
    #[serde(
        default = "default_ramp_time_ms",
        alias = "deceleration_time",
        deserialize_with = "deserialize_ramp_time_ms"
    )]
    pub deceleration_time_ms: u16,
    pub yaw_control: YawConfig,
}

fn default_ramp_time_ms() -> u16 {
    DEFAULT_RAMP_TIME_MS
}

fn deserialize_ramp_time_ms<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let value = f64::deserialize(deserializer)?;
    // Migrate legacy second-based values (e.g. 0.25) to integer milliseconds.
    let milliseconds = if value.fract() != 0.0 {
        if !(0.0..=LEGACY_MAX_RAMP_TIME_SECONDS).contains(&value) {
            return Err(D::Error::custom(
                "legacy ramp time must be 0..=0.25 seconds",
            ));
        }
        value * 1000.0
    } else {
        value
    };
    if !milliseconds.is_finite() || milliseconds < 0.0 || milliseconds > MAX_RAMP_TIME_MS as f64 {
        return Err(D::Error::custom("ramp time must be 0..=5000 milliseconds"));
    }
    Ok(milliseconds.round() as u16)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            ports: Default::default(),
            motors: [(0, 1), (1, 1), (0, 2), (1, 2)].map(|(controller, channel)| Motor {
                controller,
                channel,
                direction: 1,
            }),
            power_limit: 100,
            acceleration_time_ms: DEFAULT_RAMP_TIME_MS,
            deceleration_time_ms: DEFAULT_RAMP_TIME_MS,
            yaw_control: YawConfig::default(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        self.yaw_control.validate()?;
        if !(1..=1000).contains(&self.power_limit) {
            return Err("drive power_limit must be 1..1000".into());
        }
        if self.acceleration_time_ms > MAX_RAMP_TIME_MS
            || self.deceleration_time_ms > MAX_RAMP_TIME_MS
        {
            return Err(
                "drive acceleration_time_ms and deceleration_time_ms must be 0..=5000".into(),
            );
        }
        if self.enabled
            && (self.ports.iter().any(|p| p.trim().is_empty()) || self.ports[0] == self.ports[1])
        {
            return Err("drive requires two distinct serial paths".into());
        }
        let mut seen = [[false; 2]; 2];
        for m in &self.motors {
            if m.controller > 1 || !(1..=2).contains(&m.channel) || ![-1, 1].contains(&m.direction)
            {
                return Err("motor requires controller 0/1, channel 1/2, direction -1/+1".into());
            }
            if std::mem::replace(&mut seen[m.controller][m.channel as usize - 1], true) {
                return Err("motor channels must be unique".into());
            }
        }
        Ok(())
    }

    pub fn mix(&self, throttle: f32, steering: f32) -> Result<[[i16; 2]; 2], String> {
        if ![throttle, steering]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1.0)
        {
            return Err("drive inputs must be finite and within -1..1".into());
        }
        self.validate()?;
        let left = (throttle + steering).clamp(-1.0, 1.0);
        let right = (throttle - steering).clamp(-1.0, 1.0);
        let mut result = [[0; 2]; 2];
        for (m, demand) in self.motors.iter().zip([left, right, left, right]) {
            result[m.controller][m.channel as usize - 1] =
                (demand * self.power_limit as f32 * m.direction as f32).round() as i16;
        }
        Ok(result)
    }
}

/// Fresh commands cannot revive a stopped or expired drive. Heartbeats do not
/// renew this lease: only actual drive demands do.
pub struct Drive {
    armed: bool,
    last_command: Instant,
    demand: (f32, f32),
    output: [[f32; 2]; 2],
    last_output: Instant,
    yaw: yaw::YawController,
}

impl Default for Drive {
    fn default() -> Self {
        Self {
            armed: false,
            last_command: Instant::now(),
            demand: (0.0, 0.0),
            output: [[0.0; 2]; 2],
            last_output: Instant::now(),
            yaw: Default::default(),
        }
    }
}

impl Drive {
    pub fn enable(&mut self, now: Instant) {
        self.yaw = Default::default();
        self.armed = true;
        self.last_command = now;
        self.demand = (0.0, 0.0);
        self.output = [[0.0; 2]; 2];
        self.last_output = now;
    }

    pub fn stop(&mut self) {
        self.yaw = Default::default();
        self.armed = false;
        self.demand = (0.0, 0.0);
        self.output = [[0.0; 2]; 2];
    }

    pub fn armed(&mut self, now: Instant) -> bool {
        if now.saturating_duration_since(self.last_command) >= Duration::from_millis(WATCHDOG_MS) {
            self.stop();
        }
        self.armed
    }

    pub fn command(&mut self, throttle: f32, steering: f32, now: Instant) -> Result<(), String> {
        if ![throttle, steering]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1.0)
        {
            self.stop();
            return Err("invalid drive demand".into());
        }
        if !self.armed(now) {
            return Err("drive stopped; explicit enable required".into());
        }
        self.demand = (throttle, steering);
        self.last_command = now;
        Ok(())
    }

    pub fn output(&mut self, config: &Config, now: Instant) -> Result<[[i16; 2]; 2], String> {
        self.output_with_feedback(config, now, None)
    }

    pub fn yaw_status(&self) -> YawStatus {
        self.yaw.status.clone()
    }

    pub fn output_with_feedback(
        &mut self,
        config: &Config,
        now: Instant,
        feedback: Option<YawFeedback>,
    ) -> Result<[[i16; 2]; 2], String> {
        if let Err(error) = config.validate() {
            self.stop();
            return Err(error);
        }
        if self.armed(now) {
            let steering = if config.yaw_control.enabled {
                match self
                    .yaw
                    .update(&config.yaw_control, self.demand, feedback, now)
                {
                    Ok(steering) => steering,
                    Err(error) => {
                        self.stop();
                        return Err(error);
                    }
                }
            } else {
                self.demand.1
            };
            let target = if config.yaw_control.enabled && !self.yaw.status.active {
                config.mix(0.0, 0.0)?
            } else {
                config.mix(self.demand.0, steering)?
            };
            let elapsed = now
                .saturating_duration_since(self.last_output)
                .as_secs_f32();
            self.last_output = now;
            for (channel, target) in self
                .output
                .iter_mut()
                .flatten()
                .zip(target.iter().flatten())
            {
                let target = *target as f32;
                let current = *channel;
                // Float signum is +1/-1 even for signed zero. Zero is rest,
                // not a direction: otherwise negative targets latch at zero.
                let reversing =
                    current != 0.0 && target != 0.0 && current.signum() != target.signum();
                let accelerating = !reversing && target.abs() > current.abs();
                let ramp_time_ms = if accelerating {
                    config.acceleration_time_ms
                } else {
                    config.deceleration_time_ms
                };
                let max_delta = if ramp_time_ms == 0 {
                    f32::INFINITY
                } else {
                    config.power_limit as f32 * elapsed * 1000.0 / ramp_time_ms as f32
                };
                let mut next = current + (target - current).clamp(-max_delta, max_delta);
                // Reversals first decelerate to zero; acceleration in the new
                // direction starts on a subsequent control-loop tick.
                if reversing && next.signum() != current.signum() {
                    next = 0.0;
                }
                *channel = next;
            }
            Ok(self
                .output
                .map(|channels| channels.map(|power| power.round() as i16)))
        } else {
            Ok([[0; 2]; 2])
        }
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Telemetry {
    pub encoder_counts: [i32; 2],
    pub speed_rpm: [i32; 2],
    pub faults: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_feedback_loss_latches_stop_and_needs_explicit_enable() {
        let mut c = Config::default();
        c.yaw_control.enabled = true;
        let start = Instant::now();
        let mut drive = Drive::default();
        drive.enable(start);
        drive.command(0.3, 0.3, start).unwrap();
        let f = YawFeedback {
            yaw: 0.0,
            timestamp: 1.0,
            received: start,
            epoch: 0,
        };
        assert_eq!(
            drive.output_with_feedback(&c, start, Some(f)).unwrap(),
            [[0; 2]; 2]
        );
        let t = start + Duration::from_millis(20);
        let f = YawFeedback {
            timestamp: 1.02,
            received: t,
            ..f
        };
        assert_ne!(
            drive.output_with_feedback(&c, t, Some(f)).unwrap(),
            [[0; 2]; 2]
        );
        assert!(drive.output_with_feedback(&c, t, None).is_err());
        assert!(!drive.armed(t));
        assert!(drive.command(0.3, 0.3, t).is_err());
        assert_eq!(
            drive.output_with_feedback(&c, t, Some(f)).unwrap(),
            [[0; 2]; 2]
        );
        assert!(!drive.yaw_status().active);
    }

    #[test]
    fn mixing_mapping_and_cap() {
        let mut c = Config::default();
        c.motors[1].direction = -1;
        assert_eq!(c.mix(1.0, 1.0).unwrap(), [[100, 100], [0, 0]]);
        assert_eq!(c.mix(1.0, 0.0).unwrap(), [[100, 100], [-100, 100]]);
        for value in [f32::NAN, f32::INFINITY, 1.1] {
            assert!(c.mix(value, 0.0).is_err());
        }
    }

    #[test]
    fn rejects_bad_mapping() {
        let mut c = Config::default();
        c.motors[1] = c.motors[0].clone();
        assert!(c.validate().is_err());
        c.motors[1].controller = 99;
        assert!(c.validate().is_err());
    }

    #[test]
    fn acceleration_and_deceleration_are_ramped_and_stop_is_immediate() {
        let start = Instant::now();
        let mut config = Config::default();
        config.acceleration_time_ms = 250;
        config.deceleration_time_ms = 250;
        let mut drive = Drive::default();
        drive.enable(start);
        drive.command(1.0, 0.0, start).unwrap();

        assert_eq!(drive.output(&config, start).unwrap(), [[0; 2]; 2]);
        assert_eq!(
            drive
                .output(&config, start + Duration::from_millis(25))
                .unwrap(),
            [[10, 10], [10, 10]]
        );
        assert_eq!(
            drive
                .output(&config, start + Duration::from_millis(275))
                .unwrap(),
            [[100, 100], [100, 100]]
        );

        drive
            .command(0.0, 0.0, start + Duration::from_millis(275))
            .unwrap();
        assert_eq!(
            drive
                .output(&config, start + Duration::from_millis(300))
                .unwrap(),
            [[90, 90], [90, 90]]
        );
        drive.stop();
        assert_eq!(
            drive
                .output(&config, start + Duration::from_millis(301))
                .unwrap(),
            [[0; 2]; 2]
        );
    }

    #[test]
    fn rejects_ramp_times_above_five_seconds() {
        let mut config = Config::default();
        config.acceleration_time_ms = 5001;
        assert!(config.validate().is_err());
        config.acceleration_time_ms = 5000;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rover16_all_directions_ramp_from_rest() {
        let mut config = Config::default();
        for motor in &mut config.motors {
            motor.direction = if motor.controller == 0 { -1 } else { 1 };
        }
        for (throttle, steering, expected) in [
            (1.0, 0.0, [[-10, -10], [10, 10]]),
            (-1.0, 0.0, [[10, 10], [-10, -10]]),
            (0.0, 1.0, [[-10, -10], [-10, -10]]),
            (0.0, -1.0, [[10, 10], [10, 10]]),
        ] {
            let start = Instant::now();
            let mut drive = Drive::default();
            drive.enable(start);
            drive.command(throttle, steering, start).unwrap();
            assert_eq!(drive.output(&config, start).unwrap(), [[0; 2]; 2]);
            assert_eq!(
                drive
                    .output(&config, start + Duration::from_millis(25))
                    .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn reversal_crosses_zero_then_accelerates_negative() {
        let start = Instant::now();
        let config = Config::default();
        let mut drive = Drive::default();
        drive.enable(start);
        drive.command(1.0, 0.0, start).unwrap();
        assert_eq!(
            drive
                .output(&config, start + Duration::from_millis(250))
                .unwrap(),
            [[100; 2]; 2]
        );
        drive
            .command(-1.0, 0.0, start + Duration::from_millis(250))
            .unwrap();
        assert_eq!(
            drive
                .output(&config, start + Duration::from_millis(375))
                .unwrap(),
            [[50; 2]; 2]
        );
        assert_eq!(
            drive
                .output(&config, start + Duration::from_millis(500))
                .unwrap(),
            [[0; 2]; 2]
        );
        assert_eq!(
            drive
                .output(&config, start + Duration::from_millis(525))
                .unwrap(),
            [[-10; 2]; 2]
        );
    }

    #[test]
    fn timeout_latches_and_reenable_starts_at_zero() {
        let now = Instant::now();
        let mut drive = Drive::default();
        assert!(drive.command(1.0, 0.0, now).is_err());
        drive.enable(now);
        drive.command(1.0, 0.0, now).unwrap();
        assert_eq!(
            drive
                .output(&Config::default(), now + Duration::from_millis(250))
                .unwrap(),
            [[100; 2]; 2]
        );
        let later = now + Duration::from_millis(WATCHDOG_MS);
        assert_eq!(
            drive.output(&Config::default(), later).unwrap(),
            [[0; 2]; 2]
        );
        assert!(drive.command(1.0, 0.0, later).is_err());
        drive.enable(later);
        assert_eq!(
            drive.output(&Config::default(), later).unwrap(),
            [[0; 2]; 2]
        );
    }

    #[test]
    fn bad_command_disarms() {
        let now = Instant::now();
        let mut drive = Drive::default();
        drive.enable(now);
        assert!(drive.command(f32::NAN, 0.0, now).is_err());
        assert!(!drive.armed(now));
    }
}
