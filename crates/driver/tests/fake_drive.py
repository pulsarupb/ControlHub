"""Exercise the real serial worker using two local PTYs, never physical devices."""
import json
import os
from pathlib import Path
import pty
import select
import socket
import subprocess
import tempfile
import threading
import time
import tty
import unittest
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


class Controller:
    def __init__(self):
        self.master, slave = pty.openpty()
        tty.setraw(slave)
        self.path = os.ttyname(slave)
        os.close(slave)
        self.commands = []
        self.power = [0, 0]
        self.stopped = True
        self.running = True
        self.delay_enable = False
        self.enabling = threading.Event()
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()

    def run(self):
        pending = b""
        while self.running:
            try:
                if not select.select([self.master], [], [], .02)[0]:
                    continue
                pending += os.read(self.master, 4096)
                while b"\r" in pending:
                    raw, pending = pending.split(b"\r", 1)
                    command = raw.decode("ascii")
                    self.commands.append(command)
                    if command.startswith("!G "):
                        _, channel, value = command.split()
                        self.power[int(channel) - 1] = int(value)
                    elif command == "!EX":
                        self.stopped = True
                    elif command == "!MG":
                        self.enabling.set()
                        if self.delay_enable:
                            time.sleep(.04)  # Within the protocol's 80 ms deadline.
                        self.stopped = False
                    if command.startswith(("!", "^")):
                        reply = "+"
                    else:
                        prefix = command[1:].split()[0]
                        value = {"RWD": "500", "MXMD": "0:0", "MMOD": "0",
                                 "FF": "16" if self.stopped else "0", "C": "0", "S": "0"}[prefix]
                        reply = f"{prefix}={value}"
                    os.write(self.master, (reply + "\r").encode("ascii"))
            except OSError:
                time.sleep(.01)  # Slave may not have been opened yet.

    def close(self):
        if self.running:
            self.running = False
            self.thread.join(timeout=1)
            os.close(self.master)


class PairedDriveTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.controllers = [Controller(), Controller()]
        with socket.socket() as reserve:
            reserve.bind(("127.0.0.1", 0))
            port = reserve.getsockname()[1]
        self.url = f"http://127.0.0.1:{port}"
        root = Path(__file__).resolve().parents[3]
        self.process = subprocess.Popen(
            [str(root / "target/debug/driver"), "--bind", f"127.0.0.1:{port}",
             "--config", str(Path(self.directory.name) / "config.json")],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.wait(lambda: self.api("/api/health")[0] == 200)
        _, self.config = self.api("/api/config")
        self.config["drive"].update(enabled=True, ports=[c.path for c in self.controllers])
        self.assertEqual(self.api("/api/config", self.config, "PUT")[0], 200)
        self.wait(lambda: self.drive()["connected"])

    def tearDown(self):
        self.process.terminate()
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        for controller in self.controllers:
            controller.close()
        self.directory.cleanup()

    def api(self, path, body=None, method=None):
        data = None if body is None else json.dumps(body).encode()
        request = Request(self.url + path, data=data, method=method,
                          headers={"content-type": "application/json"})
        try:
            with urlopen(request, timeout=2) as response:
                return response.status, json.load(response)
        except HTTPError as error:
            return error.code, json.load(error)

    def drive(self):
        return self.api("/api/status")[1]["devices"]["drive"]

    def wait(self, predicate):
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline:
            try:
                if predicate():
                    return
            except (URLError, ConnectionError):
                pass
            time.sleep(.01)
        self.fail(f"fake-device condition timed out: {self.drive()}; commands: {[c.commands[-12:] for c in self.controllers]}")

    def test_disconnect_stops_both_and_reconnect_stays_disarmed(self):
        self.assertEqual(self.api("/api/drive/enable", {}, "POST")[0], 200)
        self.assertEqual(self.api("/api/control", {"throttle": 1, "steering": 0})[0], 200)
        self.wait(lambda: all(c.power == [100, 100] for c in self.controllers))
        self.controllers[1].close()
        self.wait(lambda: self.controllers[0].stopped and not self.drive()["data"]["armed"])
        self.assertEqual(self.controllers[0].power, [0, 0])
        self.controllers[1] = Controller()
        self.config["drive"]["ports"][1] = self.controllers[1].path
        self.assertEqual(self.api("/api/config", self.config, "PUT")[0], 200)
        self.wait(lambda: self.drive()["connected"])
        self.assertFalse(self.drive()["data"]["armed"])
        self.assertEqual(self.api("/api/control", {"throttle": 1, "steering": 0})[0], 400)
        self.assertEqual(self.api("/api/drive/enable", {}, "POST")[0], 200)
        self.assertEqual(self.api("/api/control", {"throttle": .5, "steering": 0})[0], 200)
        self.wait(lambda: all(c.power == [50, 50] for c in self.controllers))
        self.api("/api/stop", {}, "POST")
        self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))

    def test_stop_during_enable_cannot_release_late_motion(self):
        self.controllers[1].delay_enable = True
        self.api("/api/drive/enable", {}, "POST")
        self.assertTrue(self.controllers[1].enabling.wait(timeout=2))
        self.api("/api/stop", {}, "POST")
        self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))
        self.assertFalse(self.drive()["data"]["armed"])

    def test_stale_demand_stops_hardware_and_requires_reenable(self):
        self.api("/api/drive/enable", {}, "POST")
        self.api("/api/control", {"throttle": 1, "steering": 0})
        self.wait(lambda: all(c.power == [100, 100] for c in self.controllers))
        self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))
        self.api("/api/heartbeat", {}, "POST")
        self.assertEqual(self.api("/api/control", {"throttle": 1, "steering": 0})[0], 400)
        self.api("/api/drive/enable", {}, "POST")
        self.wait(lambda: all(not c.stopped for c in self.controllers))
        self.assertTrue(all(c.power == [0, 0] for c in self.controllers))


if __name__ == "__main__":
    unittest.main()
