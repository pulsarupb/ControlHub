"""ZED capture, tracking and bounded map publication. No Flask or serial reader."""
import base64
import queue
import time

from settings import init_parameters, tracking_parameters, mapping_parameters
from mesh_export import export_mesh


def pose_data(pose):
    return {"translation": [float(v) for v in pose.get_translation().get()],
            "orientation": [float(v) for v in pose.get_orientation().get()]}


def map_data(mesh, np, limit=20000):
    vertices, colors = [], []
    for chunk in mesh.chunks:
        if len(chunk.vertices):
            v = np.asarray(chunk.vertices, dtype=np.float32)[:, :3]
            c = np.asarray(chunk.colors, dtype=np.uint8)
            if c.ndim != 2 or len(c) != len(v) or c.shape[1] < 3:
                c = np.full((len(v), 3), 128, dtype=np.uint8)
            vertices.append(v)
            colors.append(c[:, :3])
    if not vertices:
        return [], []
    vertices, colors = np.concatenate(vertices), np.concatenate(colors)
    valid = np.isfinite(vertices).all(axis=1)
    vertices, colors = vertices[valid], colors[valid]
    step = max(1, (len(vertices) + limit - 1) // limit)
    return vertices[::step].round(4).tolist(), colors[::step].tolist()


def run(cfg, commands, emit):
    # Delayed imports allow transport/parameter tests on machines without the SDK.
    import cv2
    import numpy as np
    import pyzed.sl as sl

    camera = sl.Camera()
    try:
        code = camera.open(init_parameters(sl, cfg["init"]))
        if code != sl.ERROR_CODE.SUCCESS:
            raise RuntimeError(f"ZED open failed: {code}")
        code = camera.enable_positional_tracking(tracking_parameters(sl, cfg["tracking"]))
        if code != sl.ERROR_CODE.SUCCESS:
            raise RuntimeError(f"ZED tracking failed: {code}")
        mapping = cfg["mapping"]["enabled"]
        if mapping:
            code = camera.enable_spatial_mapping(mapping_parameters(sl, cfg["mapping"]))
            if code != sl.ERROR_CODE.SUCCESS:
                raise RuntimeError(f"ZED mapping failed: {code}")
        runtime, pose, sensors = sl.RuntimeParameters(), sl.Pose(), sl.SensorsData()
        image, depth = sl.Mat(), sl.Mat()
        mesh = sl.Mesh() if cfg["mapping"]["map_type"] == "MESH" else sl.FusedPointCloud()
        info = camera.get_camera_information()
        path, points, colors = [], [], []
        last_publish = last_map = 0.0
        grab_failures = 0
        odometry_epoch = 0

        while True:
            try:
                while True:
                    command = commands.get_nowait()
                    if command["type"] == "shutdown":
                        return
                    if command["type"] == "generate_mesh":
                        try:
                            if not mapping or cfg["mapping"]["map_type"] != "MESH":
                                raise RuntimeError("Enable ZED spatial mapping with map_type MESH first")
                            snapshot = sl.Mesh()
                            code = camera.extract_whole_spatial_map(snapshot)
                            if code != sl.ERROR_CODE.SUCCESS:
                                raise RuntimeError(f"ZED mesh extraction failed: {code}")
                            exported = export_mesh(snapshot, np, cv2, cfg["mapping"]["save_texture"])
                            emit({"type": "mesh", "mesh": exported, "error": None})
                        except Exception as error:
                            emit({"type": "mesh", "mesh": None, "error": str(error)})
                    if command["type"] == "reset":
                        odometry_epoch += 1
                        if mapping:
                            camera.disable_spatial_mapping()
                        code = camera.reset_positional_tracking(sl.Transform())
                        if code != sl.ERROR_CODE.SUCCESS:
                            raise RuntimeError(f"ZED reset failed: {code}")
                        mesh.clear()
                        path, points, colors = [], [], []
                        if mapping:
                            code = camera.enable_spatial_mapping(mapping_parameters(sl, cfg["mapping"]))
                            if code != sl.ERROR_CODE.SUCCESS:
                                raise RuntimeError(f"ZED mapping restart failed: {code}")
                        last_map = 0.0
            except queue.Empty:
                pass

            for key, value in cfg["runtime"].items():
                setattr(runtime, key, value)
            if camera.grab(runtime) != sl.ERROR_CODE.SUCCESS:
                grab_failures += 1
                if grab_failures >= 30:
                    raise RuntimeError("ZED grab failed repeatedly; reconnect required")
                time.sleep(0.01)
                continue
            grab_failures = 0
            captured_at = time.time()
            tracking = camera.get_position(pose)
            current_pose = pose_data(pose) if tracking == sl.POSITIONAL_TRACKING_STATE.OK else None
            # Small control message on EVERY frame, independent of 10 Hz UI/map updates.
            emit({"type": "odometry", "orientation": current_pose["orientation"] if current_pose else None,
                  "timestamp": camera.get_timestamp(sl.TIME_REFERENCE.IMAGE).get_nanoseconds() / 1e9,
                  "captured_at": captured_at, "epoch": odometry_epoch})
            now = time.monotonic()
            if now - last_publish < 0.1:
                continue
            last_publish = now
            if current_pose:
                path.append(current_pose)
                path = path[-3000:]
            imu_data = None
            if camera.get_sensors_data(sensors, sl.TIME_REFERENCE.IMAGE) == sl.ERROR_CODE.SUCCESS:
                imu = sensors.get_imu_data()
                if imu.is_available:
                    imu_data = {"acceleration": [float(v) for v in imu.get_linear_acceleration()],
                                "angular_velocity": [float(v) for v in imu.get_angular_velocity()]}
            if mapping and now - last_map >= 3:
                mesh.clear()
                camera.extract_whole_spatial_map(mesh)
                points, colors = map_data(mesh, np)
                last_map = now
            state = {"camera_ready": True, "camera_model": str(info.camera_model),
                     "serial": str(info.serial_number), "pose": current_pose, "path": path,
                     "tracking_state": str(tracking).split(".")[-1],
                     "mapping_state": str(camera.get_spatial_mapping_state()).split(".")[-1] if mapping else "OFF",
                     "fps": float(camera.get_current_fps()), "imu": imu_data,
                     "points": points, "colors": colors, "error": None}
            emit({"type": "state", "state": state})
            camera.retrieve_image(image, sl.VIEW.LEFT)
            color = cv2.cvtColor(image.get_data(), cv2.COLOR_BGRA2BGR)
            for kind, frame in (("color", color),):
                ok, jpeg = cv2.imencode(".jpg", frame, [cv2.IMWRITE_JPEG_QUALITY, 55])
                if ok:
                    emit({"type": "image", "kind": kind, "jpeg": base64.b64encode(jpeg).decode("ascii")})
            if cfg["runtime"]["enable_depth"]:
                camera.retrieve_measure(depth, sl.MEASURE.DEPTH)
                normalized = np.nan_to_num(np.clip(depth.get_data() / 5.0, 0, 1) * 255).astype(np.uint8)
                frame = cv2.applyColorMap(normalized, cv2.COLORMAP_TURBO)
                ok, jpeg = cv2.imencode(".jpg", frame, [cv2.IMWRITE_JPEG_QUALITY, 60])
                if ok:
                    emit({"type": "image", "kind": "depth", "jpeg": base64.b64encode(jpeg).decode("ascii")})
    finally:
        camera.close()
