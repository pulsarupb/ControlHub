use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fix {
    /// None for No Fix: never publish zero coordinates as a valid position.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude_msl: Option<f64>,
    pub quality: u8,
    pub satellites: u8,
    pub hdop: Option<f64>,
    /// Host reception epoch, not an inferred date from GGA time-of-day.
    pub received_at: f64,
    pub latitude_std: f64,
    pub longitude_std: f64,
    pub altitude_std: f64,
}

impl Fix {
    pub fn valid(&self) -> bool {
        matches!(self.quality, 1..=5) && self.latitude.is_some() && self.longitude.is_some()
    }
}

fn coordinate(raw: &str, hemisphere: &str, latitude: bool) -> Result<f64, String> {
    let value: f64 = raw.parse().map_err(|_| "invalid coordinate")?;
    let degrees = (value / 100.0).floor();
    let minutes = value - degrees * 100.0;
    let limit = if latitude { 90.0 } else { 180.0 };
    if !value.is_finite()
        || value < 0.0
        || !(0.0..60.0).contains(&minutes)
        || degrees + minutes / 60.0 > limit
    {
        return Err("coordinate out of range".into());
    }
    let sign = match (latitude, hemisphere) {
        (true, "N") | (false, "E") => 1.0,
        (true, "S") | (false, "W") => -1.0,
        _ => return Err("invalid hemisphere".into()),
    };
    Ok(sign * (degrees + minutes / 60.0))
}

/// None for unrelated sentences; malformed GGA is Some(Err). Checksum required.
pub fn parse_gga(line: &str) -> Option<Result<Fix, String>> {
    let raw = line.strip_prefix('$')?;
    if !raw.split(',').next()?.ends_with("GGA") {
        return None;
    }
    Some((|| {
        let (body, checksum) = raw.split_once('*').ok_or("missing checksum")?;
        if checksum.len() != 2
            || u8::from_str_radix(checksum, 16).ok() != Some(body.bytes().fold(0, |a, b| a ^ b))
        {
            return Err("bad NMEA checksum".into());
        }
        let f: Vec<_> = body.split(',').collect();
        if f.len() < 11 {
            return Err("short GGA sentence".into());
        }
        let quality: u8 = f[6].parse().map_err(|_| "invalid fix quality")?;
        if quality > 8 {
            return Err("unknown fix quality".into());
        }
        let satellites = f[7].parse().map_err(|_| "invalid satellite count")?;
        let optional = |s: &str| -> Result<Option<f64>, String> {
            if s.is_empty() {
                return Ok(None);
            }
            s.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .map(Some)
                .ok_or("invalid numeric field".into())
        };
        let hdop = optional(f[8])?;
        if hdop.is_some_and(|v| v < 0.0) {
            return Err("negative HDOP".into());
        }
        let (latitude, longitude) = if quality == 0 {
            (None, None)
        } else {
            (
                Some(coordinate(f[2], f[3], true)?),
                Some(coordinate(f[4], f[5], false)?),
            )
        };
        let altitude_msl = optional(f[9])?;
        if altitude_msl.is_some() && f[10] != "M" {
            return Err("altitude must be meters MSL".into());
        }
        // Same heuristic as ZED-Test, explicitly an estimate, not receiver covariance.
        let (horizontal, vertical) = match quality {
            1 => (2.5, 4.0),
            2 => (0.8, 1.5),
            4 => (0.02, 0.04),
            5 => (0.2, 0.4),
            _ => (100.0, 150.0),
        };
        let scale = hdop.filter(|v| (0.5..=20.0).contains(v)).unwrap_or(1.0);
        Ok(Fix {
            latitude,
            longitude,
            altitude_msl,
            quality,
            satellites,
            hdop,
            received_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            latitude_std: horizontal * scale,
            longitude_std: horizontal * scale,
            altitude_std: vertical * scale.max(1.0),
        })
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sentence(body: &str) -> String {
        format!("${body}*{:02X}", body.bytes().fold(0, |a, b| a ^ b))
    }
    #[test]
    fn hemispheres_and_accuracy() {
        let fix = parse_gga(&sentence(
            "GNGGA,123519,4807.038,S,01131.000,W,4,12,0.9,545.4,M,46.9,M,,",
        ))
        .unwrap()
        .unwrap();
        assert!((fix.latitude.unwrap() + 48.1173).abs() < 1e-6);
        assert!((fix.longitude.unwrap() + 11.5166667).abs() < 1e-6);
        assert!(fix.valid());
        assert!((fix.latitude_std - 0.018).abs() < 1e-8);
    }
    #[test]
    fn no_fix_is_not_a_zero_position() {
        let fix = parse_gga(&sentence("GPGGA,123519,,,,,0,00,99.9,,M,,M,,"))
            .unwrap()
            .unwrap();
        assert!(!fix.valid());
        assert_eq!(fix.latitude, None);
    }
    #[test]
    fn rejects_bad_checksum_and_coordinates() {
        assert!(parse_gga("$GPGGA,1*00").unwrap().is_err());
        assert!(
            parse_gga(&sentence("GPGGA,1,9160,N,01131,E,4,12,1,1,M,,M,,"))
                .unwrap()
                .is_err()
        );
        assert!(parse_gga("$GPRMC,unrelated").is_none());
    }
}
