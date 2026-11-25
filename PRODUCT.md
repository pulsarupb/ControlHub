# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

- **Drivers:** team members who operate the rover in real time, using drive and arm controls, live camera and sensor views, and safety stops.
- **Remote observers:** team members who watch rover status and sensor data from elsewhere, and rarely issue commands.
- The product is a **developer tool, used by coders**. Its users are technical and comfortable with configuration, telemetry and hardware terms.

## Product Purpose

Panels is the cross-platform dashboard for the ERC rover (Rover 1.6). It connects to `driver`, a native Rust API server running on the Jetson, and presents drive, arm, RTK, ZED, OAK-D, D435, device status and emergency stop in one workspace.

Success means the driver can move the rover safely and the observer can understand its state without ambiguity. Hardware state must never be misrepresented.

## Positioning

[Open. Inferred from the README, to confirm with the user: a user-arranged widget grid over a local rover API, with independent hardware crates, no ROS, no autonomy, and simulation that labels every simulated device.]

## Operating Context

- **Live teleoperation:** drive is controlled by a gamepad or on-screen bars and joystick over a persistent `/api/ws` WebSocket channel (control and arm jog/keepalive frames at 20 Hz, `drive` status at 10 Hz, `devices` status at 2 Hz, immediate `stopped` events). Closing the control link stops drive immediately (`control link closed`), and drive and arm also stop when the client loses focus or becomes hidden, recording every stop reason.
- **Commissioning and configuration:** devices are enabled and configured in Settings. Config is JSON with a credentials sidecar. Hardware validation follows `docs/robot-validation.md`.
- **Devices and inputs:** laptops with touchscreens and a gamepad; Android phones; Nintendo Switch input (native `switch-input` plugin).
- **Connection:** the rover API is reached on the rover LAN (`Local`, hotspot `Pulsar-Rover`) or in simulation (`Dev`).
- **Development:** the app is built and run locally with Bun/Vite and Tauri, and checked with `bun run check` and `bun test`.

## Capabilities and Constraints

- **Hardware:** one Roboteq drive pair (four motors, tank drive, optional ZED yaw PI); Kinova Gen3 Lite arm with joint and Cartesian control, IK preview and Kortex ReachPose; RTK receiver with NTRIP correction forwarding; ZED camera (video, depth, pose, point cloud, map reset); three OAK-D cameras; D435 camera; Jetson host power off/reboot.
- **Gamepad:** one controller with an explicit Drive or Arm target. Bindings, dead zone and speed caps are configurable.
- **Simulation:** available with `--simulate`. It never opens hardware or contacts an NTRIP caster, never fabricates GPS fixes, camera measurements or IK solutions, and labels every simulated device.
- **Safety language:** software Stop is not a certified physical E-stop. IK is not collision checking. RTK connected does not imply RTK Fixed. Accuracy is a heuristic, not a measurement.
- **Security constraint:** the API has no authentication and is for a trusted rover LAN only.
- **Terminology:** "drive", "arm", "Stop", "enable", "Local", "Dev", "simulation". Do not replace them with friendlier synonyms that change meaning.
- **Undecided:** none recorded.

## Brand Commitments

- Keep the existing **Panels** logo.
- Add a small **driftlab.ro** logo as a co-brand mark. The driftlab.ro logo asset is not in the repository and must be supplied by the user before it can be used.

## Evidence on Hand

- README, `RELEASE.md`, `USBS.md`, and `docs/` (`robot-validation.md`, `rover16-implementation.md`, `zed-yaw-control.md`, `oak-cameras.md`, `d435-camera.md`, `provenance.md`).
- Logos and icons: `logo.png` (repo root) and `app/src-tauri/icons/`. Which file is the Panels logo should be confirmed.
- Kinova model assets: `app/static/kinova-models`. Map assets: `app/static/maps`.
- Absent and not to be fabricated: customer or user testimonials, performance benchmarks, and the driftlab.ro logo asset.

## Product Principles

- **Stop wins.** Motion ends on stop, stale command, focus loss or controller failure. Nothing resumes without an explicit enable.
- **Never misrepresent hardware.** Simulated, disconnected and unvalidated states are labeled. A connection is not a fix, and a stream is not a measurement.
- **Dense and precise for technical users.** Show the state and the reason, not decoration.
- **Configurable workspace.** Widgets, workspaces and device settings are user-arranged. Defaults should be usable without setup.
- **Inputs are equal.** Gamepad, touchscreen, phone and Switch controls must all reach the same drive and arm behavior.
