#!/bin/sh
set -e

REPO="pulsarupb/ControlHub"

echo "Fetching latest release from $REPO..."

URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" \
  | grep "browser_download_url.*driver-linux" \
  | head -1 \
  | cut -d '"' -f 4)

if [ -z "$URL" ]; then
  echo "Error: Could not find a driver-linux asset in the latest release."
  exit 1
fi

echo "Downloading driver server..."
curl -L -o driver "$URL"
chmod +x driver

echo "Done. Installed $(pwd)/driver"
echo "Run ./driver to start the server."
