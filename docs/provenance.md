# Extracted source and assets

- `crates/kinova-control`: native Rust client and six required Kortex protobuf
  files from `Kinova-VLA/crates/kinova-control`. No application, AI or inference
  code was copied. Vendor BSD terms are in `proto/LICENSE` (from Kinova's
  [ros_kortex license](https://github.com/Kinovarobotics/ros_kortex/blob/noetic-devel/LICENSE)).
- `app/static/kinova-models`: Gen3 Lite URDF and STL geometry from the existing
  `Kinova-VLA/crates/test-kinova/src/static/models` viewer, originally Kinova
  description assets. Kinova BSD terms accompany the assets. The URDF's embedded
  ROS control declarations are inert: only visual geometry and joints are parsed;
  no ROS plugin is loaded and embedded connection parameters are not used.
- `app/src/lib/rover/arm-model.ts`: small typed adaptation of that viewer's URDF
  parsing, joint/mimic updates and gripper visual offset. Uses the existing Three.js
  dependency, not a second bundled copy or a browser IK implementation.
  The draggable tool target retains the source viewer's Three.js `XYZ` Euler
  convention; numeric Kortex theta fields pass through unchanged. URDF origins
  independently use their specified fixed-axis RPY convention. Validate combined
  tool rotations against live feedback before executing dragged orientations.
- `crates/roboteq`: new Rust implementation, referencing the serial commands in
  `rover-r15-software/.../sdc2130_driver/protocol.py` and `worker.py`; no ROS nodes,
  commissioning or identity framework copied.
- `crates/rtk`: Rust NMEA/NTRIP implementation based on `RTK-Test/main.py`; accuracy
  heuristics reference `ZED-Test/rtk_gps.py`. No Python GNSS serial owner remains.
- `crates/zed/sidecar`: parameter/camera/tracking/mapping behavior adapted from
  `ZED-Test/zed_slam_ui.py`. No Flask server, GNSS ingestion, fixture or fusion.

The sibling repositories were not modified. Vendor SDKs remain separately
installed runtime prerequisites, not vendored binaries or Cargo dependencies.
