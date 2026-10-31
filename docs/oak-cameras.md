# Three OAK-D cameras

Add **OAK-D Cameras (3)** from the widget picker, or use a new **Arm and Sensors**
workspace. One resizable panel displays all three slots side by side; narrow panels
stack them vertically. Existing saved workspaces are not overwritten.

In **Settings → OAK-D Cameras**, enable capture and set camera names, FPS (1–15),
and optional full MXIDs. Blank IDs discover cameras in sorted device-ID order.
Assignments remain stable during unplug/replug within one sidecar session. Pin
the IDs displayed in the panel to preserve positions across server/config restarts.
Only the configured/discovered three cameras are opened. Devices start disabled.

Each view independently selects color or colorized **disparity**. This visualization
is normalized per frame, not metric depth; magenta means invalid disparity. The
min/average/max distance statistics come from DepthAI's full-frame depth ROI and
are displayed in meters. Rotation defaults to 180 degrees, matching OakD-Test,
and can be disabled per camera. These cameras do not supply ZED tracking/yaw.

## Host requirements

- USB access to all three cameras (DepthAI udev permissions on native Linux).
- `uv`, Python 3.10+, and compatible host NumPy/OpenCV, e.g.
  `sudo apt install python3-numpy python3-opencv libusb-1.0-0` on Ubuntu.
- Stop OakD-Test or any other process holding these devices before enabling.

The `crates/oakd` Rust library owns configuration, snapshots, frame decoding and
sidecar supervision; `crates/driver/src/oak.rs` integrates it with the server state.
The server supervises `crates/oakd/sidecar/main.py`; uv creates a separate
system-site-packages environment and installs the locked DepthAI 3.8 dependencies.
Use `--oak-sidecar /installed/oak-sidecar/main.py` for native installed deployments;
keep `main.py`, `camera.py`, `pyproject.toml`, and `uv.lock` together. The release
installer, native launcher, Dockerfile and Compose include this path. Docker uses
the existing USB device mount/privileged configuration and host-compatible
NumPy/OpenCV from the image. Initial dependency installation requires network access.

Capture uses bounded latest-frame queues and three independent camera threads.
Unplugging one camera clears its view and retries without stopping the others.
Frames older than three seconds are rejected; simulation never opens hardware or
fabricates images. `GET /api/status` includes `devices.oak.data.cameras`; JPEGs
are available at `GET /api/oak/frame/{0|1|2}/{color|depth}`.

## Verification

```sh
cargo test -p oakd -p driver
python3 -m unittest discover -s crates/oakd/sidecar -v
cd app
bun run check
bun run build
```

On hardware, verify all three IDs and both views, resize the panel, unplug/replug
each camera, save/reload pinned IDs and rotation, and confirm ZED remains running.
