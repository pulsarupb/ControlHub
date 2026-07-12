# ControlHub - Panels ERC Edition

[![Release](https://img.shields.io/github/v/release/pulsarupb/ControlHub)](https://github.com/pulsarupb/ControlHub/releases)

Remote control and monitoring dashboard for the Pulsar rover. Two components work together:

- **Panels** - cross-platform desktop dashboard (Windows / Linux). Joystick, minimap, motor telemetry, gamepad support, and a customizable widget grid.
- **driver** - Rust server that runs on the rover (Jetson). Communicates with four Moteus brushless motor controllers over CAN-FD, handles tank-drive control, autonomous point-following, and WiFi hotspot management.

---

## Quick Start

### Operators - Using Panels

1. **Download Panels** from the [Releases page](https://github.com/pulsarupb/ControlHub/releases):
   - Linux: `.AppImage` or `.deb`
   - Windows: `.msi`
2. **Connect your computer to the rover's WiFi** - SSID `Pulsar-Rover` (password: `password`).
3. **Launch Panels** - it auto-connects to the rover at `10.42.0.1:8080`.

> If Panels can't find the rover, check you're connected to the rover's WiFi and click the connection indicator in the top bar to switch modes (Local / Dev / Remote).

### Operators - Installing the Server on the Rover

SSH into the Jetson and run:

```sh
curl -sSL https://raw.githubusercontent.com/pulsarupb/ControlHub/main/install-server.sh | bash
sudo ./driver
```

This downloads the latest `driver` binary and starts the server.

### Developers - Building from Source

**Prerequisites:**

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/)
- Linux only: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`

```sh
# Clone the repo
git clone https://github.com/pulsarupb/ControlHub.git
cd ControlHub

# Build & run the driver (simulated mode — no real motors needed)
cargo build -p driver --release
./target/release/driver --simulate

# In another terminal — build & run Panels
cd web
bun install
bun run tauri dev
```

---

## Connecting to the Rover

The dashboard has three connection modes, selectable from the connection indicator in the top bar:

| Mode       | URL                     | When to use                                                            |
| ---------- | ----------------------- | ---------------------------------------------------------------------- |
| **Local**  | `http://10.42.0.1:8080` | Connected to the rover's WiFi hotspot                                  |
| **Dev**    | `http://127.0.0.1:8080` | Driver running locally on your machine (`--simulate` or real hardware) |
| **Remote** | custom URL              | Rover is on a different network (e.g., a separate router)              |

> The rover's WiFi hotspot serves `10.42.0.1/24`. Ensure your computer's IP is in this subnet (most devices auto-assign correctly).

---

### Jetson Docker

```sh
docker compose up -d --build
```

The Compose file uses host networking and privileged device access so the driver can bind port `8080`, access CAN / serial devices, and manage the WiFi hotspot through NetworkManager.

---

## Configuration

### Chassis Config (`data/config.toml`)

The driver reads a TOML file describing the rover's physical setup. Pass a custom path with `--config` (defaults to `data/config.toml`).

```toml
[chassis]
wheel_radius_mm = 107.95
track_width_mm = 440.0
motor_rotations_per_wheel_rotation = 1.0

[motors.left_front]
id = 1
direction = 1.0

[motors.right_front]
id = 3
direction = -1.0

[motors.left_back]
id = 4
direction = -1.0

[motors.right_back]
id = 2
direction = 1.0
```

| Field                                | Description                                  |
| ------------------------------------ | -------------------------------------------- |
| `wheel_radius_mm`                    | Radius of the wheel (mm) — used for odometry |
| `track_width_mm`                     | Distance between left and right wheels (mm)  |
| `motor_rotations_per_wheel_rotation` | Gear ratio from motor to wheel               |
| `motors.*.id`                        | Moteus CAN-FD ID for each corner             |
| `motors.*.direction`                 | `1.0` or `-1.0` — flips rotation direction   |

### Driver CLI

| Argument          | Description                                               |
| ----------------- | --------------------------------------------------------- |
| _(none)_          | Normal mode — real CAN-FD motors, WiFi hotspot            |
| `--simulate`      | Simulated motors, no WiFi. Safe for laptops.              |
| `--config <path>` | Path to chassis config TOML (default: `data/config.toml`) |
