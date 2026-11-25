"""Exercise the real serial worker using two local PTYs, never physical devices."""
import base64
import hashlib
import json
import os
from pathlib import Path
import pty
import select
import shutil
import socket
import struct
import subprocess
import tempfile
import threading
import time
import tty
import unittest
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

WEBSOCKET_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


class WsClient:
    """Minimal masked WebSocket client using only the Python standard library."""

    def __init__(self, url):
        parsed = urlparse(url)
        if parsed.scheme not in ("http", "ws"):
            raise ValueError(f"unsupported WebSocket URL: {url}")
        host = parsed.hostname
        port = parsed.port or 80
        path = parsed.path or "/"
        self.socket = socket.create_connection((host, port), timeout=3)
        key = base64.b64encode(os.urandom(16)).decode("ascii")
        request = (
            f"GET {path} HTTP/1.1\r\n"
            f"Host: {host}:{port}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.socket.sendall(request.encode("ascii"))
        response = b""
        while b"\r\n\r\n" not in response:
            chunk = self.socket.recv(4096)
            if not chunk:
                raise ConnectionError("WebSocket handshake closed")
            response += chunk
        head, self.buffer = response.split(b"\r\n\r\n", 1)
        lines = head.split(b"\r\n")
        if b" 101 " not in lines[0]:
            raise ConnectionError(f"WebSocket upgrade failed: {lines[0]!r}")
        headers = {}
        for line in lines[1:]:
            name, _, value = line.partition(b":")
            headers[name.strip().lower()] = value.strip()
        expected = base64.b64encode(
            hashlib.sha1((key + WEBSOCKET_GUID).encode("ascii")).digest()
        )
        if headers.get(b"sec-websocket-accept") != expected:
            raise ConnectionError("WebSocket handshake accept mismatch")
        self.closed = False

    def _send_frame(self, payload, opcode=0x1):
        header = bytearray([0x80 | opcode])
        length = len(payload)
        if length < 126:
            header.append(0x80 | length)
        elif length < 65536:
            header.append(0x80 | 126)
            header.extend(struct.pack("!H", length))
        else:
            header.append(0x80 | 127)
            header.extend(struct.pack("!Q", length))
        mask = os.urandom(4)
        header.extend(mask)
        masked = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
        self.socket.sendall(bytes(header) + masked)

    def send_json(self, message):
        if self.closed:
            raise ConnectionError("WebSocket is closed")
        self._send_frame(json.dumps(message).encode("utf-8"))

    def _next_frame(self):
        if len(self.buffer) < 2:
            return None
        first, second = self.buffer[0], self.buffer[1]
        if not first & 0x80:
            raise ConnectionError("fragmented WebSocket frames are not supported")
        opcode = first & 0x0F
        masked = bool(second & 0x80)
        length = second & 0x7F
        offset = 2
        if length == 126:
            if len(self.buffer) < 4:
                return None
            length = struct.unpack("!H", self.buffer[2:4])[0]
            offset = 4
        elif length == 127:
            if len(self.buffer) < 10:
                return None
            length = struct.unpack("!Q", self.buffer[2:10])[0]
            offset = 10
        mask = b""
        if masked:
            if len(self.buffer) < offset + 4:
                return None
            mask = self.buffer[offset:offset + 4]
            offset += 4
        if len(self.buffer) < offset + length:
            return None
        payload = self.buffer[offset:offset + length]
        self.buffer = self.buffer[offset + length:]
        if masked:
            payload = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
        return opcode, payload

    def _consume(self, opcode, payload):
        if opcode == 0x1:
            return json.loads(payload.decode("utf-8"))
        if opcode == 0x8:
            self.closed = True
            raise ConnectionError("WebSocket closed by server")
        if opcode == 0x9:
            self._send_frame(payload, opcode=0xA)
        return None

    def poll(self):
        messages = []
        while True:
            frame = self._next_frame()
            if frame is None:
                if not select.select([self.socket], [], [], 0)[0]:
                    break
                chunk = self.socket.recv(65536)
                if not chunk:
                    self.closed = True
                    raise ConnectionError("WebSocket connection lost")
                self.buffer += chunk
                continue
            message = self._consume(*frame)
            if message is not None:
                messages.append(message)
        return messages

    def recv(self, timeout=3):
        deadline = time.monotonic() + timeout
        while True:
            frame = self._next_frame()
            if frame is None:
                remaining = deadline - time.monotonic()
                if remaining <= 0 or not select.select([self.socket], [], [], remaining)[0]:
                    raise TimeoutError("timed out waiting for WebSocket frame")
                chunk = self.socket.recv(65536)
                if not chunk:
                    self.closed = True
                    raise ConnectionError("WebSocket connection lost")
                self.buffer += chunk
                continue
            message = self._consume(*frame)
            if message is not None:
                return message

    def close(self):
        if self.closed:
            return
        try:
            self._send_frame(b"", opcode=0x8)
        except OSError:
            pass
        self.closed = True
        self.socket.close()


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
        self.watchdog = "500"
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
                        value = {"RWD": self.watchdog, "MXMD": "0:0", "MMOD": "0",
                                 "FID": "Roboteq v1.4a SDC2XXX", "TRN": "SDC2XXX:SDC2130",
                                 "DFC": "0:0", "PINA": "0:0:0:0:0", "AINA": "0:0:0:0",
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
        self.ws = WsClient(self.url + "/api/ws")
        self.next_id = 1
        self.last_drive = {
            "connected": False,
            "armed": False,
            "power": [[0, 0], [0, 0]],
            "yaw_control": {"active": False, "correction": 0},
        }
        self.wait(lambda: self.drive()["connected"])

    def tearDown(self):
        if self.ws is not None:
            self.ws.close()
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

    def _observe(self, message):
        if message.get("t") == "drive":
            self.last_drive = message

    def _pump(self):
        for message in self.ws.poll():
            self._observe(message)

    def drive(self):
        self._pump()
        return self.last_drive

    def request(self, frame):
        self.next_id += 1
        request = {**frame, "id": self.next_id}
        self.ws.send_json(request)
        while True:
            message = self.ws.recv()
            self._observe(message)
            if message.get("id") == self.next_id and message.get("t") in ("ack", "err"):
                return message

    def command(self, frame):
        return self.request(frame)["t"] == "ack"

    def enable(self):
        return self.command({"t": "enable"})

    def control(self, throttle, steering):
        return self.command({"t": "c", "th": throttle, "st": steering})

    def stop(self, reason="operator stop"):
        return self.command({"t": "stop", "r": reason})

    def heartbeat(self):
        return self.command({"t": "jog"})

    def wait(self, predicate):
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline:
            try:
                if predicate():
                    return
            except (URLError, ConnectionError, TimeoutError):
                pass
            time.sleep(.01)
        self.fail(f"fake-device condition timed out: {self.drive()}; commands: {[c.commands[-12:] for c in self.controllers]}")

    def test_disconnect_stops_both_and_reconnect_stays_disarmed(self):
        self.assertTrue(self.enable())
        self.assertTrue(self.control(1, 0))
        self.wait(lambda: all(c.power == [100, 100] for c in self.controllers))
        self.controllers[1].close()
        self.wait(lambda: self.controllers[0].stopped and not self.drive()["armed"])
        self.assertEqual(self.controllers[0].power, [0, 0])
        self.controllers[1] = Controller()
        self.config["drive"]["ports"][1] = self.controllers[1].path
        self.assertEqual(self.api("/api/config", self.config, "PUT")[0], 200)
        self.wait(lambda: self.drive()["connected"])
        self.assertFalse(self.drive()["armed"])
        self.assertFalse(self.control(1, 0))
        self.assertTrue(self.enable())
        self.assertTrue(self.control(.5, 0))
        self.wait(lambda: all(c.power == [50, 50] for c in self.controllers))
        self.assertTrue(self.stop())
        self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))

    def test_stop_during_enable_cannot_release_late_motion(self):
        self.controllers[1].delay_enable = True
        self.assertTrue(self.enable())
        self.assertTrue(self.controllers[1].enabling.wait(timeout=2))
        self.assertTrue(self.stop())
        self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))
        self.assertFalse(self.drive()["armed"])

    def test_configuration_change_stops_both(self):
        self.assertTrue(self.enable())
        self.assertTrue(self.control(1, 0))
        self.wait(lambda: all(c.power == [100, 100] for c in self.controllers))
        self.controllers[0].watchdog = "0"
        self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))
        self.wait(lambda: not self.drive()["armed"])

    def test_stale_demand_stops_hardware_and_requires_reenable(self):
        self.assertTrue(self.enable())
        self.assertTrue(self.control(1, 0))
        self.wait(lambda: all(c.power == [100, 100] for c in self.controllers))
        self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))
        self.assertTrue(self.heartbeat())
        self.assertFalse(self.control(1, 0))
        self.assertTrue(self.enable())
        self.wait(lambda: all(not c.stopped for c in self.controllers))
        self.assertTrue(all(c.power == [0, 0] for c in self.controllers))

    def test_zed_yaw_corrects_without_encoders_and_feedback_loss_stops_both(self):
        # Restart with a real supervised JSON sidecar, but no SDK/camera hardware.
        self.ws.close()
        self.process.terminate()
        self.process.wait(timeout=5)
        # Linux PTY exclusive-open state can survive the slave's process exit.
        for controller in self.controllers:
            controller.close()
        self.controllers = [Controller(), Controller()]
        self.config["drive"]["ports"] = [c.path for c in self.controllers]
        root = Path(__file__).resolve().parents[3]
        project = Path(self.directory.name)
        for name in ("pyproject.toml", "uv.lock"):
            shutil.copy(root / "crates/zed/sidecar" / name, project / name)
        mode = project / "mode"
        mode.write_text("ok")
        script = project / "fake_zed.py"
        script.write_text('''import json, math, sys, time
from pathlib import Path
json.loads(sys.stdin.readline())
start = time.monotonic()
while True:
    now = time.time()
    mode = Path(__file__).with_name("mode").read_text()
    yaw = (time.monotonic() - start) * 0.1
    print(json.dumps({"type":"odometry", "orientation": None if mode == "lost" else [0,0,math.sin(yaw/2),math.cos(yaw/2)],
                      "timestamp": now, "captured_at": now - (2 if mode == "stale" else 0), "epoch":0}), flush=True)
    time.sleep(0.02)
''')
        self.config["drive"]["yaw_control"].update(enabled=True, heading_hold=False)
        self.config["zed"].update(enabled=True, mount_configured=True)
        self.config["zed"]["settings"]["tracking"]["enable_area_memory"] = False
        (project / "config.json").write_text(json.dumps(self.config))
        self.process = subprocess.Popen(
            [*self.process.args, "--sidecar", str(script)],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.wait(lambda: self.api("/api/health")[0] == 200)
        self.ws = WsClient(self.url + "/api/ws")
        self.last_drive = {
            "connected": False,
            "armed": False,
            "power": [[0, 0], [0, 0]],
            "yaw_control": {"active": False, "correction": 0},
        }
        self.wait(lambda: self.drive()["connected"])
        self.wait(lambda: self.enable())

        def corrected():
            if not self.control(.3, 0):
                return False
            yaw = self.drive()["yaw_control"]
            return (yaw["active"] and yaw["correction"] > .005
                    and self.controllers[0].power[0] > self.controllers[1].power[0])

        self.wait(corrected)
        # Controller emulators return zero encoder counts/RPM throughout this test.
        for failure in ("lost", "stale"):
            mode.write_text(failure)
            self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))
            self.assertFalse(self.control(.3, 0))
            mode.write_text("ok")
            time.sleep(.1)
            self.assertFalse(self.drive()["armed"])
            self.wait(lambda: self.drive()["connected"])
            self.wait(lambda: self.enable())
            self.wait(corrected)
        self.assertTrue(self.stop())
        self.wait(lambda: all(c.stopped and c.power == [0, 0] for c in self.controllers))


if __name__ == "__main__":
    unittest.main()
