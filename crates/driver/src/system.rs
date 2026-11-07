#[cfg(target_os = "linux")]
use std::process::Command;

/// Host power action requested from the dashboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerAction {
    Shutdown,
    Reboot,
}

impl PowerAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shutdown => "shutdown",
            Self::Reboot => "reboot",
        }
    }

    fn logind_method(self) -> &'static str {
        match self {
            Self::Shutdown => "PowerOff",
            Self::Reboot => "Reboot",
        }
    }

    fn systemd_method(self) -> &'static str {
        match self {
            Self::Shutdown => "PowerOff",
            Self::Reboot => "Reboot",
        }
    }

    fn systemctl_verb(self) -> &'static str {
        match self {
            Self::Shutdown => "poweroff",
            Self::Reboot => "reboot",
        }
    }
}

/// Ask the host to power off or restart.
///
/// The Docker deployment runs privileged with the host system bus mounted, so
/// logind is the preferred (graceful) path. The syscall fallback also works
/// there through `CAP_SYS_BOOT`, but it is immediate and skips service
/// shutdown, so it is only used after the graceful paths fail.
#[cfg(target_os = "linux")]
pub fn request(action: PowerAction) -> Result<(), String> {
    if let Some(error) = run_dbus(
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        action.logind_method(),
        true,
    ) {
        eprintln!("system: logind {} failed: {error}", action.as_str());
    } else {
        return Ok(());
    }

    if let Some(error) = run_dbus(
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        action.systemd_method(),
        false,
    ) {
        eprintln!("system: systemd {} failed: {error}", action.as_str());
    } else {
        return Ok(());
    }

    match Command::new("systemctl")
        .args(["--no-block", action.systemctl_verb()])
        .output()
    {
        Ok(output) if output.status.success() => return Ok(()),
        Ok(output) => eprintln!(
            "system: systemctl {} failed: {}",
            action.as_str(),
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => eprintln!("system: cannot run systemctl: {error}"),
    }

    // Last resort for the privileged container. Sync first so at least the
    // driver's own buffers are on disk; services are not stopped gracefully.
    eprintln!(
        "system: falling back to immediate {} syscall; filesystems may not unmount cleanly",
        action.as_str()
    );
    unsafe { libc::sync() };
    let command = match action {
        PowerAction::Shutdown => libc::RB_POWER_OFF,
        PowerAction::Reboot => libc::RB_AUTOBOOT,
    };
    if unsafe { libc::reboot(command) } == 0 {
        return Ok(());
    }

    Err(format!(
        "host {} failed: {}; the driver needs host power permission (privileged container or root/polkit)",
        action.as_str(),
        std::io::Error::last_os_error()
    ))
}

#[cfg(target_os = "linux")]
fn run_dbus(destination: &str, path: &str, method: &str, interactive: bool) -> Option<String> {
    let mut command = Command::new("dbus-send");
    command.args([
        "--system",
        "--print-reply",
        "--reply-timeout=5000",
        &format!("--dest={destination}"),
        path,
        &format!("{destination}.Manager.{method}"),
    ]);
    if interactive {
        // logind's PowerOff/Reboot take an `interactive` boolean.
        command.arg("boolean:false");
    }

    match command.output() {
        Ok(output) if output.status.success() => None,
        Ok(output) => Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        Err(error) => Some(error.to_string()),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn request(_action: PowerAction) -> Result<(), String> {
    Err("host power control is only supported on Linux".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_action_names_match_host_interfaces() {
        assert_eq!(PowerAction::Shutdown.as_str(), "shutdown");
        assert_eq!(PowerAction::Reboot.as_str(), "reboot");
        assert_eq!(PowerAction::Shutdown.logind_method(), "PowerOff");
        assert_eq!(PowerAction::Reboot.logind_method(), "Reboot");
        assert_eq!(PowerAction::Shutdown.systemd_method(), "PowerOff");
        assert_eq!(PowerAction::Reboot.systemd_method(), "Reboot");
        assert_eq!(PowerAction::Shutdown.systemctl_verb(), "poweroff");
        assert_eq!(PowerAction::Reboot.systemctl_verb(), "reboot");
    }
}
