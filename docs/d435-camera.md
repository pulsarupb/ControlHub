# D435 gripper camera

Add **D435 Gripper Camera** from the Panels widget picker. The resizable color
view includes a centered crosshair toggle and automatic reconnect.

ControlHub captures the USB camera directly through the local `crates/d435`
crate, adapted from Kinova-VLA's camera detection/capture code. **No VLM service,
Ollama, inference, or RealSense SDK is required.** Plug the D435 into the rover
and run the driver normally. Simulation never opens the camera.

The crate auto-detects the RealSense YUYV color node, prefers 640×480 at 30 fps,
and JPEG-encodes once per captured frame. All Panels viewers share this capture,
fetching snapshots approximately 7 times/second. This is color only, not depth.
Missing/busy cameras are retried; frames older than three seconds are rejected.

Stop other applications capturing the D435 (including Kinova VLM/test-webcam)
before using this widget: USB capture is exclusive. No `KINOVA_VLM_URL` is used.
The driver needs permission to access `/dev/video*`; the existing privileged
Docker deployment already exposes the devices.

Read-only endpoints: `/api/d435/frame/color` and `/api/d435/frame/crosshair`.
Unavailable cameras return HTTP 503. Clients use their normal ControlHub connection.

Native builds need a C compiler and libclang for V4L2 bindings. JPEG uses a
bundled mozjpeg build with optional assembly disabled (no NASM requirement).
