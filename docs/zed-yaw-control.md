# ZED yaw feedback (no motor encoders)

This opt-in mode uses ZED orientation to regulate chassis yaw rate. Wheel encoder
counts and reported RPM are not used. Forward throttle remains open-loop power;
this is neither navigation nor individual-wheel slip estimation.

## Configuration

Existing configurations remain open-loop. The API supplies a default
`drive.yaw_control` object, editable through Settings. Leave `enabled=false`
until the hardware checks below pass.

Requirements:

- Enable ZED and verify its rigid mounting transform; set `mount_configured=true`
  only after measurement. `mount_rotation_deg` is camera-to-rover roll, pitch,
  yaw, applied as `Rz(yaw) Ry(pitch) Rx(roll)`, matching the map widget. The SDK
  uses `RIGHT_HANDED_Z_UP` (x right, y forward, z up), **not** an optical frame or
  the `Z_UP_X_FWD` convention. Account for this when defining the rover frame.
- Set ZED tracking `enable_area_memory=false` to prevent map loop-closure
  corrections from feeding the controller; `set_as_static` must also be false.
  Configuration validation enforces these requirements.
- Prefer IMU fusion on, pose smoothing off, and spatial mapping off while tuning.
  Map extraction and image encoding still run on the SDK thread: if either blocks
  capture longer than the feedback timeout, the drive deliberately stops.
- Verify motor mapping/directions and the existing RoboteQ watchdog and E-stop.
  The controllers stay in open-loop power mode; the yaw loop runs on the Jetson.

Default `drive.yaw_control` (all rates in rad/s, accelerations in rad/s²):

```json
{
  "enabled": false,
  "steering_yaw_sign": -1,
  "max_rate_rad_s": 0.6,
  "max_accel_rad_s2": 0.6,
  "kp": 0.15,
  "ki": 0.03,
  "correction_limit": 0.2,
  "filter_time_ms": 100,
  "feedback_timeout_ms": 250,
  "max_measured_rate_rad_s": 3.0,
  "heading_hold": true,
  "heading_kp": 1.0,
  "steering_deadband": 0.03
}
```

These are starting values, not commissioned gains. Positive steering makes the
left wheels faster than the right, normally a clockwise turn: hence sign `-1`
for right-handed Z-up yaw. Verify this on the actual chassis. An incorrect sign
creates positive feedback. Do not change the sign to conceal incorrect motor wiring.

## Control behavior

1. Each successful camera frame publishes a small `odometry` message before UI,
   mapping, and image work, with SDK frame time, host capture time and reset epoch.
   Non-OK tracking publishes no orientation. This is separate from the 10 Hz UI.
2. Rust removes the full mounting rotation, unwraps successive yaw differences,
   and estimates yaw rate using **camera frame time**, not network/loop timing.
   A first-order low-pass filter smooths measured rate. Two distinct frames are
   required after enabling; motor targets stay zero while priming.
3. Steering requests yaw rate, with acceleration-limited target changes. Target
   rate supplies feedforward; PI supplies a bounded normalized steering correction.
   `kp` scales rad/s error, `ki` its time integral; `correction_limit` is a fraction
   of the configured drive power limit, not an additional power allowance.
4. At neutral steering while moving forward **or backward**, heading hold captures
   the current heading and turns wrapped heading error into a bounded yaw-rate
   target. Deliberate steering releases that heading. At full joystick neutral,
   PI and heading hold reset: there is no autonomous rotation to maintain heading.
5. Conditional integration prevents windup at correction, steering, and wheel-mix
   saturation. Existing motor acceleration/deceleration ramps, reversal-through-zero,
   power caps and command watchdog still apply. The underlying tank mixer clips at
   its limits, so steering authority can be reduced at high throttle.

The Wheels / controllers panel and `/api/status` drive data expose measured and
target yaw rate, correction, active state and heading-hold state.

## Failure behavior

With yaw feedback enabled, arming requires fresh measured ZED odometry. Missing,
stale, invalid, frozen or backwards-timestamped feedback, a reset epoch change,
or an implausible yaw jump latches the drive stopped. Tracking loss and ZED
sidecar exit also stop the drive. There is **no automatic open-loop fallback**.

`feedback_timeout_ms` applies to host acquisition age as well as elapsed time
since reception. `max_measured_rate_rad_s` rejects sudden orientation changes;
it is a plausibility bound, not a guaranteed physical speed limiter. Small tracking
errors below that bound remain possible. The SDK and camera latency must be
measured on hardware; acquisition timestamps cannot eliminate that latency.

ZED reset and changes to ZED settings/mount stop an active feedback-controlled
drive. Explicit re-enable is required after valid feedback returns. Manual stop,
E-stop, serial failures and command expiry retain their existing behavior. Sensor
samples never renew the operator command lease. Simulation does not invent ZED
measurements and therefore cannot arm this mode through the normal API.

## Hardware commissioning

1. Have a working physical E-stop, an operator holding the deadman, a clear area,
   and conservative drive power and yaw-rate limits. Do not test near people.
2. With drive stopped, physically turn the rover counterclockwise. Verify measured
   base yaw increases and that rate returns to zero when stationary. Check forward,
   reverse, tilted-camera mounting, and ±π crossing.
3. Begin with `ki=0`, `heading_hold=false`, low `kp`, and a small correction limit.
   Apply short, low-speed steering commands. Confirm the measured response has
   the requested sign before allowing sustained feedback. Stop immediately if
   corrections reinforce the error.
4. Tune P for a damped response; add a small I only to remove persistent yaw-rate
   error. Increase the filter only as needed: filtering adds delay. Check both
   directions, turn-in-place, reversals, and unequal traction. Avoid prolonged
   stall/slip: yaw feedback cannot create traction or replace current protection.
5. Enable heading hold and verify straight forward/reverse travel. Check target
   slew and existing power ramps; adjust these separately from PI gains.
6. While commanding low-speed motion, cover/disconnect the camera, stop the sidecar,
   request a ZED reset, stop operator commands and trigger E-stop. Verify both
   controllers stop, restored tracking cannot restart motion, and re-enable starts
   from zero with a fresh heading/integrator.

Record actual capture intervals and acquisition-to-control latency under full
Jetson load. Do not increase the timeout simply to conceal a blocked capture thread.

## Hardware-free checks

```sh
cargo test -p roboteq -p zed -p driver
uv run --project crates/zed/sidecar --locked -- python -m unittest discover -s crates/zed/sidecar -v
```

Tests cover yaw wrapping, mounting transforms, reduced turn response, saturation,
neutral behavior, feedback faults, explicit re-enable and API reset/arming gates.
They do not validate traction dynamics, SDK latency, real motor directions or gains.
