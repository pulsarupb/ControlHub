//! Minimal synchronous RoboteQ SDC2130 control. No chassis, web or ROS dependency.
mod protocol;
pub use protocol::Controller;

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub const WATCHDOG_MS: u64 = 500;

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
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=1000).contains(&self.power_limit) {
            return Err("drive power_limit must be 1..1000".into());
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
}

impl Default for Drive {
    fn default() -> Self {
        Self {
            armed: false,
            last_command: Instant::now(),
            demand: (0.0, 0.0),
        }
    }
}

impl Drive {
    pub fn enable(&mut self, now: Instant) {
        self.armed = true;
        self.last_command = now;
        self.demand = (0.0, 0.0);
    }

    pub fn stop(&mut self) {
        self.armed = false;
        self.demand = (0.0, 0.0);
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
        if self.armed(now) {
            config.mix(self.demand.0, self.demand.1)
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
    fn timeout_latches_and_reenable_starts_at_zero() {
        let now = Instant::now();
        let mut drive = Drive::default();
        assert!(drive.command(1.0, 0.0, now).is_err());
        drive.enable(now);
        drive.command(1.0, 0.0, now).unwrap();
        assert_ne!(drive.output(&Config::default(), now).unwrap(), [[0; 2]; 2]);
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
