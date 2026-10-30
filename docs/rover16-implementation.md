# Rover 1.6 implementation and commit map

## Agreed boundaries

- Work only in ControlHub; the sibling repositories are reference material.
- Four reusable libraries under `crates/`: `roboteq`, `kinova-control`, `rtk`, `zed`.
- No legacy CAN motor backend or residual runtime configuration. No ROS requirement.
- No VLA, AI, language agent, inference server or model dependencies.
- One native Linux server on the Jetson owns all devices. ZED uses a supervised
  Python/pyzed sidecar; only the Rust RTK library opens the GNSS serial port.
  **Scope update:** RTK and ZED localization are independent; no GNSS fusion,
  fix/fixture forwarding, or covariance compatibility layers in this version.
  ZED publishes native right-handed Z-up meters; rendering performs a proper
  coordinate rotation. Camera pose is not silently relabeled as rover-base pose.
- Two USB RoboteQ SDC2130 controllers, four configurable channels. Open-loop power
  control, configurable cap, startup stopped, explicit enable, command timeout,
  hardware watchdog, paired-controller stop on failure, explicit enable after fault.
  No commissioning framework, identity gates or wheel-derived rover pose.
- Kinova manual joint/Cartesian control, gripper, home, faults, measured 3D state,
  numeric and draggable tool target, robot IK preview, explicit execution. An IK
  solution is **not** collision checking.
- RTK-Test supplies NTRIP/NMEA logic; ZED-Test supplies camera/tracking/mapping
  behavior. RTK2go `rtk2go.com:2101`, mount point `SemaAgoraRobotics`, is configurable.
- Independent subsystem enable/disable and failures. The two motor controllers
  together form one drive subsystem. Global software Stop stops drive and arm.
- Drive and arm can operate independently. One gamepad has an explicit Drive/Arm
  target toggle, not simultaneous routing of the same input.
- Map displays ZED pose/trajectory/point cloud. Separate RTK and wheel-status widgets;
  video and arm widgets are independently placeable. No autonomous navigation.
- No robot is currently available. Hardware validation remains pending even when
  all local tests pass. Never substitute simulation silently for missing hardware.

## Proposed commits (do not commit automatically)

These are reviewable logical boundaries, in dependency order, not a claim that
each stage is already complete. Keep each commit green before moving on.
Dependency-coupled renames/imports may need to travel together when staging these
logical groups. No commits have been created or pushed.

| # | Suggested commit | Contents | Acceptance gate |
|---|---|---|---|
| 1 | `docs: define rover16 scope and robot validation gates` | This map and `robot-validation.md`; reference provenance and explicit exclusions | Review against agreed scope; no motion commands in automated setup |
| 2 | `feat(roboteq): add minimal independent serial motor library` | Serial framing/ACK/query, two-channel commands, watchdog, bounded tank mixing, mapping validation, measured channel status | Unit tests for malformed replies, mapping, limits, stale commands, paired stop, recovery |
| 3 | `feat(kinova): extract native Kortex control library` | Protobuf transport/session/RPC/IK from sibling repository; no web or AI dependencies | Transport/router/IK tests; builds without robot or Python |
| 4 | `feat(rtk): add NMEA receiver and NTRIP correction library` | GGA parsing/checksums, fix validity/age, serial owner, bounded caster handshake, reconnect, correction forwarding | NMEA hemisphere/no-fix/checksum tests; local fake-caster tests, no public network required |
| 5 | `feat(zed): add supervised modular pyzed sidecar` | Rust lifecycle/protocol; Python camera, settings, streaming, spatial mapping; bounded histories and messages; no RTK fusion | SDK-free protocol/lifecycle tests and Python tests; missing SDK produces actionable offline status |
| 6 | `refactor(driver): integrate independent hardware services` | Unified validated config, device status, drive/arm control, IK preview/execute, global Stop, device-only restart | API tests reject invalid/disabled/offline operations; explicit simulation; stale/fault traffic never re-enables motion |
| 7 | `refactor: remove legacy motor backend and wheel localization` | Remove old dependency/files/CLI/config/schema/labels/docs; update lockfile and migration notice | Search tracked text for obsolete backend references; driver tests pass; no wheel pose integration |
| 8 | `feat(panels): add hardware settings and live status widgets` | Motors/arm/RTK/ZED/gamepad settings, wheel status, RTK pose, independent enable indicators, persistent config | Svelte/TypeScript checks and production build; save/reload and validation errors tested |
| 9 | `feat(panels): add ZED map video and Kinova IK visualization` | Color/depth feeds, ZED map/point cloud, arm measured/preview state, draggable target, solve then explicit execute | Browser smoke test in simulation; resize/dispose, offline/stale states; target edits invalidate preview |
| 10 | `feat(input): share gamepad between drive and arm` | Y/triangle target toggle; LB/L1 hold-to-jog; drive left Y/right X; arm XY/Z/yaw, D-pad pitch/roll, triggers gripper; B/circle global Stop; configurable bindings/speeds | Deadman release, target switch, gamepad loss and page blur zero old demand; UI legend matches mapping |
| 11 | `test: add integration coverage and deployment runbook` | Fake devices/sidecar/caster, explicit simulation smoke flow, native Jetson setup, final results and remaining hardware gates | Run local gates below; record exact results; robot gates remain unchecked until performed |

## Local validation gates

1. Format/check/test the four libraries and driver without connected hardware.
2. Python sidecar unit tests run without importing pyzed at module import time.
3. Frontend typecheck and production build; inspect browser console in simulation.
4. API tests cover non-finite/out-of-range commands, disabled devices, global Stop,
   explicit re-enable, config validation, and stale IK previews.
5. Start explicitly with `--simulate`; confirm every simulated device is labeled.
6. Missing hardware/SDK tests use fake endpoints; never open random serial devices.
7. Inspect changes and dependency tree for unwanted AI/ROS/legacy motor code.

## Review and stop rules

Ask the owner before changing this scope, replacing vendor APIs with guessed
behavior, weakening motion protections, changing SDK semantics, or executing real
motion. Do not reset networking/search paths, write controller flash, auto-start a
service, deploy remotely, or commit without authorization.

The migration changes the hardware configuration schema. Preserve a user's old
configuration rather than silently interpreting old motor IDs as serial channels.

## File-level review map

- 1: `docs/rover16-implementation.md`, `docs/robot-validation.md`, `docs/provenance.md`.
- 2–5: one directory each under `crates/{roboteq,kinova-control,rtk,zed}`; workspace
  membership in `Cargo.toml`, with the corresponding `Cargo.lock` dependency hunks.
- 6: `crates/driver/src/{main,config,state,api,motor,arm,sensors}.rs`,
  `crates/driver/Cargo.toml`.
  `wifi.rs` only has two equivalent lint cleanups; hotspot setup is now opt-in.
- 7: old backend dependency removal; deleted
  `crates/driver/src/{localizer,simulator,urdf}.rs`; frontend driver-specific paths
  renamed to `app/src/lib/rover/` and `rover-types.ts`.
- 8: `app/src/lib/{SettingsOverlay,ConfigFields}.svelte`, `config-api.ts`,
  `hardware-types.ts`, `hardware.svelte.ts`, `rover-control.svelte.ts`, status widgets.
- 9: `app/src/lib/rover/{ZedMapWidget,ZedVideoWidget,KinovaViewWidget,KinovaControlsWidget}.svelte`,
  `arm-model.ts`, `app/static/kinova-models/`, widget registration/templates.
- 10: `gamepad-{control.svelte,mapping}.ts`, `widgets/GamepadWidget.svelte`, app layout.
- 11: `crates/driver/tests/fake_drive.py`, Rust/Python/frontend tests, CI workflows,
  native `start-services.sh`, README, release sidecar packaging/installer.
  `app/package.json` and `app/bun.lock` contain the approved dependency update:
  compatible Svelte/checker updates and TypeScript 6 (TS7 needs a second compiler).

## Actual local results — 2026-10-30

Implementation is present and uncommitted. This is **not** hardware acceptance.

| Check | Result |
|---|---|
| Rust tests: driver + four libraries | 38 passed |
| Strict Clippy, all targets for those five packages | Passed, no warnings |
| Python sidecar protocol tests, no SDK imported | 3 passed |
| Two fake serial controllers, actual driver and local PTYs | 3 passed: disconnect/recovery, stale-demand stop, Stop during enable |
| Local fake NTRIP caster | Passed within Rust suite: auth, binary payload, disconnect |
| Fake Python sidecar lifecycle | Passed within Rust suite: configure, event, process reaping |
| Bun gamepad mapping tests | 3 passed |
| `bun run check` | 0 errors; 8 existing accessibility/unused-CSS warnings |
| `bun run build` | Passed; Three.js chunk-size warning remains |
| Browser smoke, explicit simulation | Settings persisted; all device labels; arm meshes and two WebGL views; gripper; global Stop; no fabricated IK |
| Browser gamepad stub | Drive demand; held-deadman target switch does not command arm; release/re-press jogs arm; release sends zero; B stops both |

No real device or public NTRIP caster was contacted. Expected negative API results
(offline/no-frame/IK unavailable/commands after Stop) are not JavaScript crashes.
No collision/path validation, live IK execution, USB firmware/watchdog behavior,
camera SDK compatibility, live GNSS accuracy or robot performance is established.
Full desktop/mobile artifacts, cross-compilation and release publication have not
been run locally. Follow `robot-validation.md` before any physical motion.
