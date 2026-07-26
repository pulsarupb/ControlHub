#!/bin/bash
set -euo pipefail

echo "=== Starting ControlHub Services ==="

echo "Starting KasmVNC desktop..."
sudo systemctl start kasmvnc

echo "Starting driver (Docker)..."
docker compose up -d

echo ""
echo "=== All services started ==="
echo "  Driver API:  http://<rover-ip>:8080/"
echo "  Kasm Desktop: http://<rover-ip>:6901/"
