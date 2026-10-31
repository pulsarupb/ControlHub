import unittest
import json
from pathlib import Path
import subprocess
import sys

from main import assign_devices, snapshot


class CamerasTest(unittest.TestCase):
    def test_stable_discovery_and_reserved_ids(self):
        config = [{"mxid": "pinned"}, {"mxid": ""}, {"mxid": ""}]
        self.assertEqual(assign_devices(config, ["", "", ""], ["b", "a"]), ["pinned", "a", "b"])
        self.assertEqual(assign_devices(config, ["pinned", "a", "b"], ["b", "c"]), ["pinned", "a", "b"])

    def test_offline_stale_and_failure_states(self):
        self.assertFalse(snapshot({}, 10)["connected"])
        self.assertTrue(snapshot({"updated_at": 9}, 10)["connected"])
        self.assertFalse(snapshot({"updated_at": 6}, 10)["connected"])
        self.assertFalse(snapshot({"updated_at": 11}, 10)["connected"])
        self.assertFalse(snapshot({"updated_at": 9, "error": "unplugged"}, 10)["connected"])

    def test_missing_sdk_is_a_protocol_error(self):
        result = subprocess.run([sys.executable, "-S", str(Path(__file__).with_name("main.py"))],
                                input="{}\n", text=True, capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        event = json.loads(result.stdout)
        self.assertEqual(event["type"], "error")
        self.assertIn("DepthAI/USB/NumPy/OpenCV", event["error"])

    def test_no_devices_has_three_offline_slots_and_native_logs_do_not_corrupt_protocol(self):
        code = """
import json, os, sys, types
import main
class Device:
    @staticmethod
    def getAllAvailableDevices():
        os.write(1, b'native SDK diagnostic\\n')
        return []
sys.modules['depthai'] = types.SimpleNamespace(Device=Device)
main.time.sleep = lambda _: sys.exit(0)
wire = os.fdopen(os.dup(sys.stdout.fileno()), 'w', buffering=1)
main.main(wire)
"""
        config = {"fps": 10, "cameras": [{"mxid": ""} for _ in range(3)]}
        result = subprocess.run([sys.executable, "-S", "-c", code], cwd=Path(__file__).parent,
                                input=json.dumps(config) + "\n", text=True, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        event = json.loads(result.stdout)
        self.assertEqual(event["type"], "state")
        self.assertEqual(len(event["cameras"]), 3)
        self.assertTrue(all(not camera["connected"] for camera in event["cameras"]))
        self.assertIn("native SDK diagnostic", result.stderr)


if __name__ == "__main__":
    unittest.main()
