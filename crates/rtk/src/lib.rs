//! NMEA fixes and optional NTRIP correction transport. No camera or web dependency.
mod nmea;
mod ntrip;
pub use nmea::{Fix, parse_gga};
pub use ntrip::NtripClient;

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct NtripConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub mount_point: String,
    pub username: String,
    pub password: String,
}

impl Default for NtripConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "rtk2go.com".into(),
            port: 2101,
            mount_point: "SemaAgoraRobotics".into(),
            username: "rtk2go@rtk2go.com".into(),
            password: String::new(),
        }
    }
}

impl NtripConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.host.trim().is_empty()
            || self.port == 0
            || self.mount_point.is_empty()
            || [
                &self.host,
                &self.mount_point,
                &self.username,
                &self.password,
            ]
            .iter()
            .any(|s| s.contains(['\r', '\n']))
            || self.host.chars().any(char::is_whitespace)
            || self.mount_point.chars().any(char::is_whitespace)
        {
            return Err("invalid NTRIP host, port, mount point or credentials".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub enabled: bool,
    pub port: String,
    pub baud: u32,
    pub ntrip: NtripConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            port: String::new(),
            baud: 115_200,
            ntrip: NtripConfig::default(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.baud == 0 || (self.enabled && self.port.trim().is_empty()) {
            return Err("RTK requires a serial port and positive baud".into());
        }
        self.ntrip.validate()
    }
}

/// The single owner of GNSS serial I/O. Correction writes and NMEA reads occur
/// on the same worker; ZED receives parsed fixes, never a serial handle.
pub struct Receiver {
    serial: Box<dyn serialport::SerialPort>,
    line: Vec<u8>,
    discarding: bool,
    pub bytes_read: u64,
    pub parse_errors: u64,
}

impl Receiver {
    pub fn open(config: &Config) -> io::Result<Self> {
        config.validate().map_err(io::Error::other)?;
        Ok(Self {
            serial: serialport::new(&config.port, config.baud)
                .timeout(Duration::from_millis(20))
                .open()?,
            line: vec![],
            discarding: false,
            bytes_read: 0,
            parse_errors: 0,
        })
    }

    pub fn corrections(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.serial.write_all(bytes)
    }

    pub fn poll(&mut self) -> io::Result<Vec<Fix>> {
        let mut bytes = [0; 1024];
        let count = match self.serial.read(&mut bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "GNSS disconnected",
                ));
            }
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::TimedOut => return Ok(vec![]),
            Err(e) => return Err(e),
        };
        self.bytes_read += count as u64;
        let mut fixes = vec![];
        for &byte in &bytes[..count] {
            if matches!(byte, b'\r' | b'\n') {
                if !self.discarding && !self.line.is_empty() {
                    match std::str::from_utf8(&self.line).ok().and_then(parse_gga) {
                        Some(Ok(fix)) => fixes.push(fix),
                        Some(Err(_)) => self.parse_errors += 1,
                        None => {}
                    }
                }
                self.line.clear();
                self.discarding = false;
            } else if !self.discarding {
                self.line.push(byte);
                if self.line.len() > 4096 {
                    self.line.clear();
                    self.discarding = true;
                    self.parse_errors += 1;
                }
            }
        }
        Ok(fixes)
    }
}
