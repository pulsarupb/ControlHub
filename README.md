# Moteus-Testing
Testing Moteus Drivers over CAN-FD

## Quick Install

### Linux Server (one-liner)

```sh
curl -sSL https://raw.githubusercontent.com/pulsarupb/ControlHub/main/install-server.sh | bash
```

This downloads the latest `driver` server binary and places it in your current directory.

### Linux Panels Dashboard

Download the latest `.AppImage` or `.deb` from the [Releases](https://github.com/pulsarupb/ControlHub/releases) page.

### Windows Panels Dashboard

Download the latest `.msi` from the [Releases](https://github.com/pulsarupb/ControlHub/releases) page.

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Bun](https://bun.sh/)
- Linux only: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`

### Server (driver)

```sh
cargo build -p driver --release
./target/release/driver
```

### Jetson Docker

Build and start the driver container on the Jetson:

```sh
docker compose up -d --build
```

The compose service uses `restart: unless-stopped`, so Docker restarts the driver after Jetson boot once the Docker service starts. It also runs with host networking and privileged device access so the driver can bind `0.0.0.0:8080`, access CAN/serial devices, and start the WiFi hotspot through NetworkManager.

To stop automatic restarts:

```sh
docker compose down
```

### Panels Dashboard (Tauri)

```sh
cd web
bun install
bun run tauri build
```

The built bundles are in `web/src-tauri/target/release/bundle/`.
