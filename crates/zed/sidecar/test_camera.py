"""Capture/control transport checks without a camera, GPU or vendor packages."""
import json
from pathlib import Path
import queue
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

import camera


class EndFrame(Exception):
    pass


class CameraOdometryTests(unittest.TestCase):
    def capture_one(self, tracking_ok=True, reset=False):
        cfg = json.loads(Path(__file__).with_name("defaults.json").read_text())
        cfg["mapping"]["enabled"] = False
        pose = SimpleNamespace(
            get_translation=lambda: SimpleNamespace(get=lambda: [0, 0, 0]),
            get_orientation=lambda: SimpleNamespace(get=lambda: [0, 0, 0, 1]))
        device = Mock()
        device.open.return_value = device.enable_positional_tracking.return_value = 0
        device.grab.return_value = device.reset_positional_tracking.return_value = 0
        device.get_position.return_value = "OK" if tracking_ok else "SEARCHING"
        device.get_timestamp.return_value.get_nanoseconds.return_value = 1234000000
        device.get_camera_information.return_value = SimpleNamespace(camera_model="ZED2i", serial_number=1)
        sdk = SimpleNamespace(
            Camera=lambda: device, RuntimeParameters=SimpleNamespace,
            Pose=lambda: pose, SensorsData=SimpleNamespace, Mat=SimpleNamespace,
            Mesh=Mock, FusedPointCloud=Mock, Transform=SimpleNamespace,
            ERROR_CODE=SimpleNamespace(SUCCESS=0),
            POSITIONAL_TRACKING_STATE=SimpleNamespace(OK="OK"),
            TIME_REFERENCE=SimpleNamespace(IMAGE="IMAGE"))
        commands = queue.Queue()
        if reset:
            commands.put({"type": "reset"})
        events = []

        def emit(event):
            events.append(event)
            raise EndFrame()

        with patch.dict(sys.modules, {"cv2": Mock(), "numpy": Mock(),
                                      "pyzed": SimpleNamespace(sl=sdk), "pyzed.sl": sdk}), \
                patch.object(camera, "init_parameters", return_value=None), \
                patch.object(camera, "tracking_parameters", return_value=None):
            with self.assertRaises(EndFrame):
                camera.run(cfg, commands, emit)
        device.close.assert_called_once()
        device.retrieve_image.assert_not_called()
        event = events[0]
        self.assertEqual(event["type"], "odometry")
        self.assertEqual(event["timestamp"], 1.234)
        self.assertGreater(event["captured_at"], 0)
        self.assertEqual(event["epoch"], int(reset))
        return event

    def test_odometry_precedes_slow_dashboard_work(self):
        self.assertEqual(self.capture_one()["orientation"], [0.0, 0.0, 0.0, 1.0])

    def test_tracking_loss_is_explicit(self):
        self.assertIsNone(self.capture_one(tracking_ok=False)["orientation"])

    def test_reset_changes_odometry_epoch(self):
        self.capture_one(reset=True)


if __name__ == "__main__":
    unittest.main()
