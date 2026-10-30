"""JSON-lines transport only. All vendor SDK calls belong to the camera thread."""
import json
import os
import queue
import sys
import threading


def emit(output, event):
    output.write(json.dumps(event, allow_nan=False, separators=(",", ":")) + "\n")
    output.flush()


def main():
    wire = os.fdopen(os.dup(sys.stdout.fileno()), "w", buffering=1)
    # Redirect native SDK printf output as well as Python print calls.
    os.dup2(sys.stderr.fileno(), sys.stdout.fileno())
    sys.stdout = sys.stderr
    commands = queue.Queue(maxsize=16)

    def read_commands():
        for line in sys.stdin:
            if len(line) > 65536:
                continue
            try:
                commands.put(json.loads(line), timeout=1)
            except (ValueError, queue.Full):
                continue
        commands.put({"type": "shutdown"})

    threading.Thread(target=read_commands, daemon=True).start()
    try:
        command = commands.get(timeout=10)
        if command.get("type") != "configure":
            raise ValueError("first command must configure ZED")
        from camera import run
        run(command["settings"], commands, lambda event: emit(wire, event))
    except Exception as error:
        emit(wire, {"type": "error", "error": str(error)})
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
