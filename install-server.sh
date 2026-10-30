#!/bin/sh
set -e

REPO="pulsarupb/ControlHub"

ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  ASSET="driver-linux-x86_64" ;;
  aarch64) ASSET="driver-linux-aarch64" ;;
  *)
    echo "Error: Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

echo "Fetching latest release ($ARCH) from $REPO..."

URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" \
  | grep "browser_download_url.*$ASSET" \
  | head -1 \
  | cut -d '"' -f 4)

if [ -z "$URL" ]; then
  echo "Error: Could not find a $ASSET asset in the latest release."
  exit 1
fi

echo "Downloading driver server..."
curl -L -o driver "$URL"

echo "Verifying checksum..."
RELEASE_URL="${URL%/*}"
CHECKSUM_URL="$RELEASE_URL/SHA256SUMS"
expected=$(curl -sL "$CHECKSUM_URL" | grep "$ASSET" | awk '{print $1}')
if [ -z "$expected" ]; then
  echo "Warning: Could not fetch checksum, skipping verification."
else
  actual=$(sha256sum driver | awk '{print $1}')
  if [ "$expected" != "$actual" ]; then
    echo "Error: Checksum mismatch"
    echo "  expected: $expected"
    echo "  actual:   $actual"
    rm -f driver
    exit 1
  fi
  echo "Checksum verified."
fi

chmod +x driver

echo "Downloading matching ZED sidecar (Python/SDK must be installed separately)..."
SIDECAR="zed-sidecar.tar.gz"
curl -fL -o "$SIDECAR" "$RELEASE_URL/$SIDECAR"
expected=$(curl -fsL "$CHECKSUM_URL" | grep " $SIDECAR\$" | awk '{print $1}')
if [ -z "$expected" ] || [ "$expected" != "$(sha256sum "$SIDECAR" | awk '{print $1}')" ]; then
  echo "Error: ZED sidecar checksum missing or mismatched; not extracting."
  exit 1
fi
tar -xzf "$SIDECAR"

echo "Done. Installed $(pwd)/driver"
echo "Run ./driver --sidecar ./sidecar/main.py to start the server."
echo "Devices start disabled. Configure serial permissions and hardware settings before enabling."
