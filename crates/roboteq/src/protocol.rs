use crate::{Telemetry, WATCHDOG_MS};
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

/// Own this on one worker thread. At most one transaction is outstanding.
pub struct Controller<T: Read + Write> {
    io: T,
}

impl Controller<Box<dyn serialport::SerialPort>> {
    pub fn open(path: &str) -> io::Result<Self> {
        let port = serialport::new(path, 115_200)
            .timeout(Duration::from_millis(10))
            .open()?;
        // Drop sends unacknowledged stop commands. Their old ACKs must not be
        // mistaken for new configuration replies when reopening a healthy peer.
        port.clear(serialport::ClearBuffer::Input)?;
        let mut controller = Self::new(port);
        controller.install_open_loop_safety()?;
        Ok(controller)
    }
}

impl<T: Read + Write> Controller<T> {
    fn install_open_loop_safety(&mut self) -> io::Result<()> {
        self.stop()?;
        let fid = self.query("?FID", "FID")?;
        let trn = self.query("?TRN", "TRN")?;
        if !fid.to_ascii_uppercase().contains("ROBOTEQ")
            || !trn.to_ascii_uppercase().contains("SDC2130")
        {
            return Err(io::Error::other(format!(
                "unexpected controller identity: {trn} / {fid}"
            )));
        }
        // Runtime-only configuration: never save to controller flash.
        for command in ["^MMOD 1 0".into(), "^MMOD 2 0".into(), "^DFC 0 0".into()]
            .into_iter()
            .chain((1..=5).map(|channel| format!("^PINA {channel} 0")))
            .chain((1..=4).map(|channel| format!("^AINA {channel} 0")))
            .chain([format!("^RWD {WATCHDOG_MS}")])
        {
            self.command(&command)?;
        }
        self.verify_drive_mode()?;
        for (command, prefix, expected) in [
            ("~DFC", "DFC", "0:0"),
            ("~PINA", "PINA", "0:0:0:0:0"),
            ("~AINA", "AINA", "0:0:0:0"),
        ] {
            let value = self.query(command, prefix)?;
            if value != expected {
                return Err(io::Error::other(format!(
                    "{prefix} readback mismatch: {value}"
                )));
            }
        }
        self.require_faults(16)
    }

    pub fn verify_setting(&mut self, index: usize) -> io::Result<()> {
        let (command, prefix, expected) = [
            ("~RWD", "RWD", WATCHDOG_MS.to_string()),
            ("~MMOD 1", "MMOD", "0".into()),
            ("~MMOD 2", "MMOD", "0".into()),
        ][index % 3]
            .clone();
        let value = self.query(command, prefix)?;
        if value != expected {
            return Err(io::Error::other(format!(
                "{command} changed: {value}; expected {expected}"
            )));
        }
        Ok(())
    }

    fn verify_drive_mode(&mut self) -> io::Result<()> {
        for index in 0..3 {
            self.verify_setting(index)?;
        }
        if !matches!(self.query("~MXMD", "MXMD")?.as_str(), "0" | "0:0") {
            return Err(io::Error::other(
                "controller channels must be independent (MXMD all-zero)",
            ));
        }
        Ok(())
    }

    fn require_faults(&mut self, expected: i32) -> io::Result<()> {
        let value = self.query("?FF", "FF")?;
        let flags = value.parse::<i32>().map_err(io::Error::other)?;
        if flags != expected {
            return Err(io::Error::other(format!(
                "controller FF={flags} ({flags:#x}); expected {expected}"
            )));
        }
        Ok(())
    }
    pub fn new(io: T) -> Self {
        Self { io }
    }

    fn send(&mut self, command: &str) -> io::Result<()> {
        if command.is_empty()
            || !command.is_ascii()
            || command.contains(['\r', '\n', '_', '%'])
            || command.starts_with("!EES")
        {
            return Err(io::Error::other("invalid or persistent controller command"));
        }
        self.io.write_all(format!("{command}\r").as_bytes())
    }

    fn line(&mut self, deadline: Instant) -> io::Result<String> {
        let mut line = Vec::new();
        let mut byte = [0];
        while Instant::now() < deadline {
            match self.io.read(&mut byte) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "controller disconnected",
                    ));
                }
                Ok(_) => {
                    if matches!(byte[0], b'\r' | b'\n') {
                        if !line.is_empty() {
                            return String::from_utf8(line).map_err(io::Error::other);
                        }
                    } else {
                        line.push(byte[0]);
                        if line.len() > 4096 {
                            return Err(io::Error::other("oversized controller response"));
                        }
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::TimedOut
                            | io::ErrorKind::WouldBlock
                            | io::ErrorKind::Interrupted
                    ) => {}
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "controller reply timeout",
        ))
    }

    pub fn command(&mut self, command: &str) -> io::Result<()> {
        self.send(command)?;
        let deadline = Instant::now() + Duration::from_millis(80);
        loop {
            let reply = self.line(deadline)?;
            match reply.trim() {
                "+" => return Ok(()),
                value if value == command => continue,
                _ => {
                    return Err(io::Error::other(format!(
                        "unexpected reply to {command}: {reply}"
                    )));
                }
            }
        }
    }

    pub fn query(&mut self, command: &str, prefix: &str) -> io::Result<String> {
        self.send(command)?;
        let deadline = Instant::now() + Duration::from_millis(80);
        loop {
            let reply = self.line(deadline)?;
            let reply = reply.trim();
            if reply == command {
                continue;
            }
            return reply
                .strip_prefix(&format!("{prefix}="))
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    io::Error::other(format!("unexpected reply to {command}: {reply}"))
                });
        }
    }

    pub fn stop(&mut self) -> io::Result<()> {
        // Attempt every stop even when an earlier transaction fails.
        let mut result = Ok(());
        for cmd in ["!G 1 0", "!G 2 0", "!MS 1", "!MS 2", "!EX"] {
            if let Err(e) = self.command(cmd) {
                result = Err(e);
            }
        }
        result
    }

    pub fn enable(&mut self) -> io::Result<()> {
        self.verify_drive_mode()?;
        self.power([0, 0])?;
        self.command("!MG")?;
        self.require_faults(0)
    }

    pub fn power(&mut self, power: [i16; 2]) -> io::Result<()> {
        if power.iter().any(|p| !(-1000..=1000).contains(p)) {
            return Err(io::Error::other("power out of range"));
        }
        self.command(&format!("!G 1 {}", power[0]))?;
        self.command(&format!("!G 2 {}", power[1]))
    }

    pub fn telemetry(&mut self) -> io::Result<Telemetry> {
        let mut query = |cmd, prefix| {
            self.query(cmd, prefix)?
                .parse::<i32>()
                .map_err(io::Error::other)
        };
        Ok(Telemetry {
            encoder_counts: [query("?C 1", "C")?, query("?C 2", "C")?],
            speed_rpm: [query("?S 1", "S")?, query("?S 2", "S")?],
            faults: query("?FF", "FF")?,
        })
    }
}

impl<T: Read + Write> Drop for Controller<T> {
    fn drop(&mut self) {
        // No ACK wait on drop; hardware watchdog is the independent backstop.
        for cmd in ["!G 1 0", "!G 2 0", "!MS 1", "!MS 2", "!EX"] {
            let _ = self.send(cmd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Wire {
        input: io::Cursor<Vec<u8>>,
        output: Vec<u8>,
    }
    impl Read for Wire {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            self.input.read(b)
        }
    }
    impl Write for Wire {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    fn controller(reply: &str) -> Controller<Wire> {
        Controller::new(Wire {
            input: io::Cursor::new(reply.as_bytes().to_vec()),
            output: vec![],
        })
    }
    #[test]
    fn handles_echo_and_crlf() {
        let mut c = controller("!G 1 20\r\n+\r\n?C 1\r\nC=-42\r");
        c.command("!G 1 20").unwrap();
        assert_eq!(c.query("?C 1", "C").unwrap(), "-42");
        assert_eq!(c.io.output, b"!G 1 20\r?C 1\r");
    }
    #[test]
    fn rejects_bad_reply_and_persistent_writes() {
        for reply in ["-\r", "C=1\r", ""] {
            assert!(controller(reply).command("!G 1 0").is_err());
        }
        for cmd in ["", "%EESAV", "!EES", "!G 1 0\r!MG", "!G_1"] {
            assert!(controller("").command(cmd).is_err());
        }
        assert!(controller("S=1\r").query("?C 1", "C").is_err());
    }
    #[test]
    fn stop_attempts_every_channel_after_error() {
        let mut c = controller("-\r+\r+\r+\r+\r");
        assert!(c.stop().is_err());
        assert_eq!(c.io.output, b"!G 1 0\r!G 2 0\r!MS 1\r!MS 2\r!EX\r");
    }

    fn setup_reply(pina: &str) -> String {
        format!(
            "{}FID=Roboteq v1.4a\rTRN=SDC2XXX:SDC2130\r{}RWD=500\rMMOD=0\rMMOD=0\rMXMD=0:0\rDFC=0:0\rPINA={pina}\rAINA=0:0:0:0\rFF=16\r",
            "+\r".repeat(5),
            "+\r".repeat(13)
        )
    }

    #[test]
    fn initialization_verifies_input_configuration_before_arming() {
        let mut c = controller(&setup_reply("0:0:0:0:0"));
        c.install_open_loop_safety().unwrap();
        let commands = String::from_utf8(c.io.output.clone()).unwrap();
        assert!(commands.contains("^DFC 0 0\r"));
        assert!(commands.contains("^PINA 5 0\r"));
        assert!(commands.contains("^AINA 4 0\r"));
        assert!(!commands.contains("!MG"));
        let mut c = controller(&setup_reply("17:33:0:0:0"));
        assert!(
            c.install_open_loop_safety()
                .unwrap_err()
                .to_string()
                .contains("PINA")
        );
    }

    #[test]
    fn enable_rejects_changed_mode_and_reports_remaining_fault() {
        let mut c = controller("RWD=500\rMMOD=1\r");
        assert!(c.enable().is_err());
        assert!(!String::from_utf8_lossy(&c.io.output).contains("!MG"));
        let mut c = controller("RWD=500\rMMOD=0\rMMOD=0\rMXMD=0:0\r+\r+\r+\rFF=18\r");
        assert!(c.enable().unwrap_err().to_string().contains("FF=18 (0x12)"));
    }
}
