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

### Panels Dashboard (Tauri)

```sh
cd web
bun install
bun run tauri build
```

The built bundles are in `web/src-tauri/target/release/bundle/`.
