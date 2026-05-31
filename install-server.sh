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
chmod +x driver

echo "Done. Installed $(pwd)/driver"
echo "Run ./driver to start the server."
