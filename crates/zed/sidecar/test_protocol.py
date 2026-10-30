import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from main import emit


class ProtocolTests(unittest.TestCase):
    def test_one_json_line(self):
        output = io.StringIO()
        emit(output, {"type": "error", "error": "missing SDK\ninstall pyzed"})
        self.assertEqual(len(output.getvalue().splitlines()), 1)
        self.assertEqual(json.loads(output.getvalue())["type"], "error")

    def test_nonfinite_values_do_not_cross_protocol(self):
        with self.assertRaises(ValueError):
            emit(io.StringIO(), {"invalid": float("nan")})

    def test_native_diagnostics_do_not_corrupt_json_stdout(self):
        code = """import os,sys,types,main
sys.modules['camera'] = types.SimpleNamespace(run=lambda cfg,commands,emit:
    (os.write(1,b'vendor diagnostic\\n'),emit({'type':'error','error':'fake SDK'})))
raise SystemExit(main.main())
"""
        process = subprocess.run([sys.executable, "-c", code], cwd=Path(__file__).parent,
                                 input='{"type":"configure","settings":{}}\n',
                                 text=True, capture_output=True, timeout=3, check=True)
        self.assertEqual(json.loads(process.stdout)["error"], "fake SDK")
        self.assertIn("vendor diagnostic", process.stderr)


if __name__ == "__main__":
    unittest.main()
