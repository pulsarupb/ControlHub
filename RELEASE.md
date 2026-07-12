# Release Process

## Automatic (recommended)

1. **Bump the version** in `Cargo.toml` (`workspace.package.version`)
2. Commit to `main` — the `Auto Tag On Version Bump` workflow creates a `v*` tag automatically
3. The `Release` workflow triggers on the tag, builds all artifacts, generates checksums and the Tauri updater manifest, and publishes a GitHub release

## What gets released

| Asset | Platform |
|---|---|
| `driver-linux-x86_64` | Linux driver (x86_64) |
| `driver-linux-aarch64` | Linux driver (ARM64) |
| `*.AppImage` + `.sig` | Linux panels dashboard |
| `*.deb` / `*.rpm` | Linux package bundles |
| `*.msi` / `*.exe` + `.sig` | Windows panels dashboard |
| `SHA256SUMS` | Checksums for all assets |
| `updates.json` | Tauri updater manifest (committed back to `main`) |

## Post-release

The release workflow automatically commits updated `updates.json` and `web/src-tauri/tauri.conf.json` back to `main`.

## Version verification

The release workflow enforces that the git tag (`vX.Y.Z`) matches the workspace version in `Cargo.toml` (`X.Y.Z`). Mismatched tags are rejected.
