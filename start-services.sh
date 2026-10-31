#!/bin/bash
set -euo pipefail

# Native deployment; no implicit Docker, networking or desktop-service changes.
cd "$(dirname "$0")"
exec "${CONTROLHUB_DRIVER:-./target/release/driver}" \
  --config "${CONTROLHUB_CONFIG:-data/rover16.json}" \
  --sidecar "${CONTROLHUB_SIDECAR:-crates/zed/sidecar/main.py}" \
  --oak-sidecar "${CONTROLHUB_OAK_SIDECAR:-crates/oakd/sidecar/main.py}" "$@"
