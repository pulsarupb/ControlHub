use std::process::Command;

const SSID: &str = "Pulsar-Rover";
const PASSWORD: &str = "password";

pub fn ensure_hotspot() -> (String, String) {
    let iface = match find_wifi_interface() {
        Some(i) => i,
        None => {
            eprintln!("wifi: no WiFi interface found, skipping hotspot setup");
            return (SSID.to_string(), PASSWORD.to_string());
        }
    };

    let active = Command::new("nmcli")
        .args(["-t", "-f", "NAME", "c", "show", "--active"])
        .output();

    match active {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.lines().any(|line| line.trim() == SSID) {
                println!("WiFi hotspot '{SSID}' already active on {iface}");
                return (SSID.to_string(), PASSWORD.to_string());
            }
        }
        _ => {}
    }

    let result = Command::new("nmcli")
        .args([
            "device", "wifi", "hotspot", "ifname", &iface, "ssid", SSID, "password", PASSWORD,
        ])
        .output();

    match result {
        Ok(output) if output.status.success() => {
            println!("WiFi hotspot '{SSID}' started on {iface}");
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("wifi: failed to start hotspot: {stderr}");
        }
        Err(e) => {
            eprintln!("wifi: failed to run nmcli: {e}");
        }
    }

    (SSID.to_string(), PASSWORD.to_string())
}

fn find_wifi_interface() -> Option<String> {
    if let Ok(output) = Command::new("nmcli")
        .args(["-t", "-f", "TYPE,DEVICE", "d"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("wifi:") {
                    let iface = line.trim_start_matches("wifi:").trim();
                    if !iface.is_empty() {
                        return Some(iface.to_string());
                    }
                }
            }
        }
    }

    if let Ok(output) = Command::new("iw").args(["dev"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if let Some(iface) = trimmed.strip_prefix("Interface ") {
                    return Some(iface.to_string());
                }
            }
        }
    }

    None
}
