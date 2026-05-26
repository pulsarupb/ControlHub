#!/usr/bin/env bash
set -euo pipefail

CAN_IF="${1:-can0}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_RUST="${ROOT_DIR}/repo-rust"

if [[ ! -d "${REPO_RUST}" ]]; then
  echo "error: repo-rust not found at ${REPO_RUST}" >&2
  exit 1
fi

if ! ip link show "${CAN_IF}" >/dev/null 2>&1; then
  echo "error: CAN interface ${CAN_IF} does not exist" >&2
  echo "bring it up first, for example:" >&2
  echo "  sudo ip link set ${CAN_IF} type can bitrate 1000000 dbitrate 5000000 fd on" >&2
  echo "  sudo ip link set ${CAN_IF} up" >&2
  exit 1
fi

if ! ip link show "${CAN_IF}" | grep -q "state UP"; then
  echo "warning: ${CAN_IF} does not appear to be UP" >&2
  echo "try:" >&2
  echo "  sudo ip link set ${CAN_IF} up" >&2
fi

echo "Discovering moteus controllers on ${CAN_IF}..." >&2

OUTPUT="$(
  cd "${REPO_RUST}"
  tools/bazel run //lib/rust:discover -- --can-chan "${CAN_IF}" --force-transport socketcan 2>&1
)"

DEVICE_LINES="$(printf '%s\n' "${OUTPUT}" | grep '^DeviceInfo(can_id=' || true)"

if [[ -z "${DEVICE_LINES}" ]]; then
  echo "No moteus controllers found on ${CAN_IF}." >&2
  echo "Full discover output:" >&2
  printf '%s\n' "${OUTPUT}" >&2
  exit 1
fi

echo "Found CAN IDs:"
printf '%s\n' "${DEVICE_LINES}" | sed -E 's/^DeviceInfo\(can_id=([0-9]+),.*$/\1/'

echo
echo "Device details:"
printf '%s\n' "${DEVICE_LINES}"
