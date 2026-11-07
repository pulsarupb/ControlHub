# Robot validation — native Jetson

**Status: in progress. Initial development was software-only; Jetson validation is underway.**
Passing software tests does not establish hardware safety or SDK compatibility.
Use this checklist in order. Record date, operator, revision, configuration,
JetPack/ZED SDK versions, observations and logs for each stage. Stop at a failed
gate; do not skip ahead to motion. A software Stop is not a physical E-stop.

## Progress record — 2026-10-30

Revision `7c2344a` (`main`). Operator not recorded. The E-stop-ready observer was
confirmed. Jetson environment: Ubuntu 24.04.4, JetPack 7.2.1, aarch64, Python
3.12.3, Rust 1.98.1, uv 0.12.11, ZED SDK 5.4.1. In the sidecar venv, NumPy
1.26.4, OpenCV 4.8.0 and `pyzed.sl` imported successfully. System Python selects
NumPy 2.5.3, which is incompatible with the installed OpenCV; NumPy 1.26.4 was
installed only in the sidecar venv. No camera was opened.

`cargo build -p driver`, `cargo test -p driver` (13 passed), strict driver Clippy,
sidecar protocol tests (3 passed), frontend checks and frontend build passed.
Frontend check/build emitted existing warnings. `cargo test --workspace` was also
attempted but could not build `gdk-sys` because `gdk-3.0` development files are
missing. Simulated API startup/Global Stop and native startup with every device
disabled were checked; the earlier simulated Playwright UI smoke test is recorded
in the handoff.

Read-only USB discovery found one accessible serial device:
`/dev/serial/by-id/usb-u-blox_AG_-_www.u-blox.com_u-blox_GNSS_receiver-if00`
(u-blox GNSS). No RoboteQ controllers were present in `/dev/serial/by-id/`.
RTK-only was then run at 115200 baud with NTRIP disabled: 43,638 bytes received,
zero parse errors, but the latest fix had quality 0, zero satellites, HDOP 99.99,
and no coordinates or altitude. RTK validation is blocked pending receiver/antenna
fix confirmation. No drive or arm motion was commanded.

## 0. Preparation (power to motion disabled)

- [x] Have a physical E-stop and an observer able to remove actuator power.
- [ ] Lift and secure the rover with all wheels clear; secure the arm base, clear
      its swept volume, remove payload, and keep people outside that volume.
- [ ] Review USB controller channel wiring, motor directions, Kinova model/IP,
      camera mounting, antenna offset, and intended base-frame axes.
- [ ] Back up current configuration. Do not reuse old controller IDs as USB mapping.
- [ ] Prevent other software (ROS, old test apps, another ControlHub instance) from
      owning the same serial ports, Kortex session, or ZED camera.
- [ ] Do not enable boot-time startup until all later gates pass.

## 1. Jetson environment (no actuator commands)

- [x] Record `uname -a`, architecture, JetPack, Python, Rust and ZED SDK versions.
- [x] Install uv and the vendor-supported ZED SDK/pyzed for this JetPack. Verify
      `pyzed`, OpenCV and NumPy in the sidecar's uv environment, using the exact
      SDK-matched wheel and system Python configured for the sidecar.
- [x] Build/test from source in debug mode first; run frontend checks/build.
- [x] Identify each USB device using `/dev/serial/by-id/` and `udevadm info`; use
      persistent paths, not guessed `/dev/ttyACM0` assignments. Confirm permissions.
- [ ] Configure distinct ports for controller A, controller B and GNSS. Never probe
      ports by sending motor commands to unknown devices.
- [x] Run explicit simulation and validate UI first. Then stop simulation completely.
- [x] Start native hardware mode with every hardware subsystem disabled. API and
      Panels must report devices disabled, not simulated or healthy.

## 2. Independent connectivity (still no motion)

- [ ] Enable RTK only. Verify serial/NMEA counters, fix quality, satellites, HDOP,
      timestamps/age, latitude/longitude signs and altitude against receiver data.
- [ ] Configure NTRIP caster/mount point/credentials. Confirm correction bytes
      arrive; distinguish a connected caster from an RTK Fixed solution. Disconnect
      Internet and verify reconnect and stale/no-fix reporting without fake fixes.
- [ ] Enable ZED only. Verify color/depth, FPS, device identity, tracking/IMU,
      trajectory/point cloud and camera errors. Move the unpowered rover/camera by
      hand and check axes, scale, heading and tracking-loss behavior.
- [ ] Confirm RTK owns GNSS serial exclusively. No Python serial reader may also
      open it. ZED and RTK are independent: no fixes/fixtures go into ZED.
- [ ] Confirm the map uses ZED local pose, while the RTK widget shows geographic
      position. No fused position or fused heading is claimed in this version.
- [ ] Disconnect ZED/RTK separately. Only the affected device must fail;
      other devices and API remain usable. Reconnect without enabling any motion.
- [ ] Enable Kinova connectivity with arm motion disabled. Verify measured angles,
      gripper, tool pose, state/faults and that 3D model tracks the physical arm.
- [ ] Enable drive connectivity with drive motion disabled. Confirm the controller
      command vocabulary, baud, runtime watchdog setting/readback and channel
      telemetry on the actual firmware. Do not write persistent controller flash.

## 3. Drive, wheels suspended (explicit motion approval required)

- [ ] Confirm independent controller channels (MXMD all-zero), open-loop MMOD on
      both channels, and watchdog readback. Check controller input priorities and
      disable external analog/pulse command sources before motion. The minimal
      driver does not reconfigure all input mappings or run commissioning.

- [ ] Set a low open-loop power cap suitable for this hardware. Confirm global Stop
      and physical E-stop before enabling. Joystick power is **not** speed control.
- [ ] Enable at neutral. Apply a short low-power pulse; verify each corner/channel
      and direction. Stop before editing mapping or directions. Never assume defaults
      represent the actual wiring. No wheel-derived pose should appear on the map.
- [ ] Verify neutral, joystick release and gamepad deadman release stop demand.
- [ ] Interrupt command traffic, close/blur the client, unplug the gamepad and
      disconnect Wi-Fi separately. Confirm bounded stop latency; record it. Losing
      window focus must stop both drive and arm immediately.
- [ ] Disconnect one controller with observer ready. Confirm both sides stop and
      require explicit enable after recovery; new commands/heartbeats cannot re-arm.
      Confirm the UI shows the controller fault as the drive stop reason.
- [ ] Test server termination and USB failure. Verify hardware watchdog stops motors
      even if software cannot send zero. Keep clear of wheels throughout.
- [ ] Test invalid input and settings save while enabled. No restart may resume old
      demand or bypass a previous Stop. Verify global Stop without waiting for UI polls.

## 4. Arm, clear workspace (separate motion approval required)

- [ ] Verify multi-axis tool orientation against Kortex measured feedback and the
      physical arm, not only a single-axis rotation. The draggable target preserves
      the old viewer's XYZ Euler convention; do not assume that validates firmware
      conventions or the model's tool-frame offset.

- [ ] Use low translation/angular/joint speeds. Verify arm Stop, global Stop and
      physical E-stop before each first motion test.
- [ ] Test tiny joint jogs, then base-frame XYZ and orientation jogs. Confirm units
      (meters/degrees), signs, joint order, deadman behavior and commanded limits.
- [ ] Test gripper with no object and fingers/personnel clear; then built-in Home
      only after confirming its path is unobstructed.
- [ ] Solve IK for the current measured pose: solving/dragging must produce **no
      motion**. Check returned joints against the live pose and preview model.
- [ ] Edit a target after solving: old preview must become non-executable. Test
      unreachable targets, stale previews and changed measured starting position.
- [ ] Execute a nearby approved target explicitly. Kortex IK is not collision
      checking; inspect the entire path, not only the destination.
- [ ] Test Stop during a target move, gamepad release, client loss, arm transport
      loss and config change. Old requests must not replay after reconnect.
- [ ] While stopped, test mode toggling: Y/triangle switches input target and zeros
      the previous target. LB/L1 must be held to jog. Verify B/circle stops both.

## 5. Integrated and ground tests

- [ ] Verify all configurable gamepad bindings against the connected controller's
      standard mapping. Check legend, dead zones, sign conventions and speed caps.
- [ ] Confirm independent drive/arm enabling and global Stop while both operate in
      a controlled, supervised setup. A ZED/RTK failure must not disable unrelated
      manual controls; a drive controller failure must stop the paired drive.
- [ ] Check saved settings survive restart, disabled devices stay unopened, secrets
      are not printed, simulation is unmistakable and offline data is not shown fresh.
- [ ] Test shutdown and reboot from the rover connection overlay. Motion must stop
      first, the Jetson must power off/restart, and Panels must report offline then
      online. Verify the driver has host power permission and that a failed request
      leaves the rover online.
- [ ] Run color/depth streams, mapping, RTK/NTRIP and arm viewer together. Record
      CPU/GPU/RAM, temperatures, stream latency and command/Stop latency under load.
- [ ] Only after suspended tests pass, perform low-power ground driving outdoors
      with the arm stowed and observer/E-stop. No autonomous driving is included.

## Acceptance record

| Gate | Date / operator | Revision / config | Evidence / measured latency | Pass / blocker |
|---|---|---|---|---|
| Environment | | 7c2344a / default-disabled config | JetPack 7.2.1; sidecar imports; driver tests/build and frontend checks/build pass | Pass |
| RTK/NTRIP | | 7c2344a / RTK-only, 115200, NTRIP off | 43,638 bytes, 0 parse errors; quality 0, 0 satellites, HDOP 99.99, no coordinates/altitude | Blocker: no fix |
| ZED tracking/mapping | | | | Pending |
| Drive suspended | | | | Pending |
| Arm manual/IK | | | | Pending |
| Gamepad/global Stop | | | | Pending |
| Integrated load/ground | | | | Pending |
