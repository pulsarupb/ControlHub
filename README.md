# ControlHub — Rover 1.6

Panels is the cross-platform rover dashboard. `driver` is the native Rust API
server running on one Jetson. Hardware support lives in four independent crates:

| Crate | Responsibility |
|---|---|
| `roboteq` | Two USB SDC2130 controllers, four mapped motors, open-loop tank drive |
| `kinova-control` | Native Kortex TCP, measured state, joint/Cartesian control and IK |
| `rtk` | Serial NMEA receiver and configurable NTRIP correction forwarding |
| `zed` | Supervised, modular Python/pyzed camera, depth, tracking and mapping |

No ROS, AI/inference or autonomous driving is included. **RTK and ZED localization
are independent; no GNSS fixes or fixtures are sent to ZED.** Wheel encoders are
telemetry only, never a source of map pose.

## Local development

Prerequisites: stable Rust, Bun, uv, and Python 3. No robot, GPU SDK, Python
packages or `protoc` installation is needed for the hardware-free tests.

From the repository root:

```sh
cargo test -p driver -p roboteq -p kinova-control -p rtk -p zed
uv run --project crates/zed/sidecar --locked -- python -m unittest discover -s crates/zed/sidecar -v
cargo build --locked -p driver
python3 crates/driver/tests/fake_drive.py -v
cargo run -p driver -- --simulate --config /tmp/opencode/controlhub-dev.json
```

In another terminal, from `app/`:

```sh
bun install --frozen-lockfile
bun test
bun run check
bun run dev
```

Choose **Dev** in the connection dialog. Open Settings to enable/configure devices.
In simulation, use distinct placeholder drive ports (e.g. `simulation-A` and
`simulation-B`). Simulation never opens hardware or contacts an NTRIP caster.
It does not fabricate GPS fixes, camera measurements or IK solutions. Every
simulated device is labeled. Real IK requires the connected arm.

## Native Jetson deployment

1. Install uv using Astral's [official instructions](https://docs.astral.sh/uv/getting-started/installation/)
   and ensure it is on the PATH of the account that runs the driver.
2. Install the vendor-supported ZED SDK/pyzed for the Jetson's actual JetPack
   version, including its compatible NumPy and OpenCV. The sidecar's uv
   environment inherits system packages to preserve the SDK's native ABI. If the
   SDK provides a matching wheel under `/usr/local/zed`, install the single wheel
   matching the active Python/architecture into the uv environment from the repo root:

   ```sh
   uv venv --allow-existing --system-site-packages --no-managed-python --python /usr/bin/python3 crates/zed/sidecar/.venv
   uv pip install --python crates/zed/sidecar/.venv/bin/python --no-deps /usr/local/zed/pyzed-*.whl
   ```
3. Build with `cargo build -p driver` or use the matching release binary.
4. Keep `crates/zed/sidecar/{main,camera,settings}.py`, `pyproject.toml`, and
   `uv.lock` together. The Rust supervisor prepares and runs the project with uv.
   Release installs include all of these files in `zed-sidecar.tar.gz`.
5. Run as an account permitted to use the chosen serial devices. Prefer
   `/dev/serial/by-id/` paths; do not assume ACM numbering or motor directions.
6. Start the API without changing networking:

   ```sh
   cargo run -p driver -- --config data/rover16.json
   ```

   A downloaded binary uses `./driver --config data/rover16.json --sidecar ./sidecar/main.py`.
   `--hotspot` explicitly opts into the existing NetworkManager hotspot helper.

**Do not deploy to the robot or enable motion until following
[`docs/robot-validation.md`](docs/robot-validation.md). No Jetson was available
during development.** Docker files are retained for existing core deployments;
the full ZED/JetPack container runtime is not provisioned or validated here.

## Configuration and controls

Configuration is JSON, default `data/config.json`. Missing files start with every
hardware device disabled. The new schema has `drive`, `kinova`, `rtk`, `zed` and
`gamepad` sections, all editable in Settings. The API rejects the old
`chassis`/`motors` schema: back it up and use a new config path rather than silently
reinterpreting old controller IDs. Saving changes only restarts affected devices;
connection changes stop their motion and never resume old demands.

- **Drive:** controller indices 0/1, channels 1/2, direction ±1. Wheel order is left
  front, right front, left back, right back. Power cap is 1–1000 controller units,
  not a guaranteed speed. Start stopped, explicitly enable, stop on stale commands
  or either controller failure, and explicitly enable again after recovery.
- **Arm:** manual joint/Cartesian jogging, gripper, Home and fault controls.
  Cartesian targets use robot-base XYZ meters and roll/pitch/yaw degrees. Drag or
  edit the target, Solve, inspect the gold IK ghost, then explicitly Execute.
  Preview expires after 15 seconds and is rejected if the arm moved. **IK is not
  collision checking.** Stop is software control, not a certified physical E-stop.
- **RTK:** defaults to `rtk2go.com:2101`, mount point `SemaAgoraRobotics`, with
  editable credentials. NTRIP is disabled until configured. A connected correction
  stream does not imply RTK Fixed. Accuracy shown is a quality/HDOP heuristic.
- **ZED:** color/depth video, pose/IMU, trajectory, point cloud and map reset.
  Native coordinates are right-handed Z-up meters. Rendering applies a proper
  rotation; a base marker appears only after explicitly configuring the measured
  camera-to-rover mount. It is not measured chassis geometry.
- **Gamepad:** one controller, explicit Drive/Arm target. Default Y/△ switches,
  LB/L1 holds to jog, B/○ stops both. Release and press LB after switching. Drive
  uses left Y/right X; arm uses left XY, right Z/yaw, D-pad pitch/roll and triggers
  for gripper. Bindings, dead zone and speed caps are configurable.

The map shows ZED data only. RTK and wheel/controller status have separate widgets;
ZED video, arm controls and arm 3D/IK target are independently placeable.

## API / safety notes

`GET /api/health`, `/api/status`, `/api/config`, `/api/zed/map`;
`PUT /api/config`; `POST /api/control`, `/api/drive/enable`, `/api/drive/stop`,
`/api/arm/action`, `/api/arm/jog`, `/api/arm/stop`, `/api/heartbeat`, `/api/zed/reset`,
`/api/stop`; `GET /api/zed/frame/{color|depth}`.

The API is intended for a trusted rover LAN; it has no authentication and must not
be exposed to the public Internet. Config files contain credentials, are written
with restrictive permissions on Unix, and passwords are redacted from GET.
Blank password fields on save retain stored secrets. NTRIP uses the existing
plain TCP/basic-auth transport; use trusted networks for correction credentials.

Drive command timeout and controller watchdog are 500 ms. Arm jogging freshness is
300 ms and client heartbeat timeout is 500 ms; Kortex's velocity-command duration
is documented as unimplemented. Physical Stop behavior on process/network loss
must be verified on the actual arm; keep a physical E-stop available.

See [`docs/rover16-implementation.md`](docs/rover16-implementation.md) for the
review/commit map and validation status. Changes are not committed automatically.
