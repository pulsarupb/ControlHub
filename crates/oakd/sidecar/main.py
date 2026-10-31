"""Bounded latest-frame capture for three independently reconnecting OAK-Ds."""
import base64
import contextlib
import json
import os
import sys
import threading
import time

from camera import capture


def assign_devices(configured, assigned, available):
    """Reserve explicit IDs even when absent; keep auto slots stable across unplugging."""
    result = [c["mxid"] or assigned[i] for i, c in enumerate(configured)]
    free = sorted(set(available) - set(result))
    for i, mxid in enumerate(result):
        if not mxid and free:
            result[i] = free.pop(0)
    return result


def snapshot(camera, now):
    fields = ("mxid", "updated_at", "error", "min_mm", "avg_mm", "max_mm")
    result = {key: camera.get(key) for key in fields}
    result["mxid"] = result["mxid"] or ""
    result["connected"] = not result["error"] and result["updated_at"] is not None and 0 <= now - result["updated_at"] < 3
    return result


def main(protocol):
    # Vendor libraries occasionally print diagnostics: stdout is protocol-only.
    os.dup2(sys.stderr.fileno(), sys.stdout.fileno())
    sys.stdout = sys.stderr
    with contextlib.redirect_stdout(sys.stderr):
        import depthai as dai
        config = json.loads(sys.stdin.readline())
        lock = threading.Lock()
        cameras = [{"error": "Waiting for camera"} for _ in range(3)]
        assigned = ["", "", ""]
        threads = [None, None, None]
        retries = [0.0, 0.0, 0.0]
        sequences = [[0, 0] for _ in range(3)]

        def emit(event):
            protocol.write(json.dumps(event, separators=(",", ":")) + "\n")
            protocol.flush()

        def worker(slot, mxid):
            def publish(kind, value):
                with lock:
                    camera = cameras[slot]
                    if kind == "metrics":
                        camera.update(value)
                    else:
                        camera[kind] = value
                        camera[kind + "_seq"] = camera.get(kind + "_seq", 0) + 1
                        camera[kind + "_at"] = time.time()
                        camera["updated_at"] = time.time()
                        camera["error"] = None
            try:
                capture(mxid, config["cameras"][slot], config["fps"], publish)
            except Exception as error:
                with lock:
                    cameras[slot] = {"mxid": mxid, "error": str(error)}
            finally:
                retries[slot] = time.monotonic() + 2

        next_scan = 0
        while True:
            now = time.monotonic()
            if now >= next_scan:
                try:
                    available = [info.getDeviceId() for info in dai.Device.getAllAvailableDevices()]
                    assigned = assign_devices(config["cameras"], assigned, available)
                    for slot, mxid in enumerate(assigned):
                        if threads[slot] is not None and threads[slot].is_alive():
                            continue
                        if now < retries[slot]:
                            continue
                        with lock:
                            cameras[slot] = {"mxid": mxid, "error": "Connecting…" if mxid in available else "Camera not found"}
                            sequences[slot] = [0, 0]
                        if mxid and mxid in available:
                            threads[slot] = threading.Thread(target=worker, args=(slot, mxid), daemon=True)
                            threads[slot].start()
                except Exception as error:
                    print(f"OAK-D discovery: {error}", file=sys.stderr)
                next_scan = now + 2
            with lock:
                data = [dict(camera) for camera in cameras]
            emit({"type": "state", "cameras": [snapshot(camera, time.time()) for camera in data]})
            for slot, camera in enumerate(data):
                if not snapshot(camera, time.time())["connected"]:
                    continue
                for index, kind in enumerate(("color", "depth")):
                    seq = camera.get(kind + "_seq", 0)
                    if seq != sequences[slot][index]:
                        sequences[slot][index] = seq
                        emit({"type": "image", "slot": slot, "kind": kind,
                              "captured_at": camera[kind + "_at"],
                              "jpeg": base64.b64encode(camera[kind]).decode("ascii")})
            time.sleep(1 / config["fps"])


if __name__ == "__main__":
    protocol = os.fdopen(os.dup(sys.stdout.fileno()), "w", buffering=1)
    try:
        main(protocol)
    except Exception as error:
        protocol.write(json.dumps({"type": "error", "error": f"OAK-D capture failed (DepthAI/USB/NumPy/OpenCV): {error}"}) + "\n")
        protocol.flush()
        sys.exit(1)
