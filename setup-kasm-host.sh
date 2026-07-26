#!/bin/bash
set -uo pipefail

# ============================================================
# ControlHub - Kasm Desktop Host Setup Script
#
# Installs KasmVNC + LXQt desktop directly on the Jetson host,
# eliminating Docker containers for the desktop.
# Run as root or with sudo.
# ============================================================

KASMVNC_VERSION="${KASMVNC_VERSION:-1.3.1}"
VNC_PASSWORD="${VNC_PASSWORD:-kasm123}"
REPO_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ $EUID -ne 0 ]]; then
    echo "This script must be run as root (sudo)."
    exit 1
fi

echo "=== ControlHub Kasm Host Setup ==="

# ------------------------------------------------------------------
# 1. Install system dependencies
# ------------------------------------------------------------------
echo "[1/7] Installing system dependencies..."
apt update
apt install -y wget curl expect

# ------------------------------------------------------------------
# 2. Install KasmVNC
# ------------------------------------------------------------------
if command -v vncserver &>/dev/null; then
    echo "[2/7] KasmVNC already installed, skipping."
else
    echo "[2/7] Installing KasmVNC v${KASMVNC_VERSION}..."
    TMPDIR=$(mktemp -d)
    cd "$TMPDIR"
    PKG="kasmvncserver_jammy_${KASMVNC_VERSION}_arm64.deb"
    wget -q "https://github.com/kasmtech/KasmVNC/releases/download/v${KASMVNC_VERSION}/${PKG}" -O kasmvnc.deb
    dpkg -i kasmvnc.deb || apt install -f -y
    cd "$REPO_DIR"
    rm -rf "$TMPDIR"
    echo "KasmVNC installed."
fi

# ------------------------------------------------------------------
# 3. Install LXQt desktop environment
# ------------------------------------------------------------------
echo "[3/7] Installing LXQt desktop environment..."
apt install -y lxqt openbox lxqt-panel lxqt-session \
    pcmanfm-qt qterminal pavucontrol

# ------------------------------------------------------------------
# 4. Configure KasmVNC for root user
# ------------------------------------------------------------------
echo "[4/7] Configuring KasmVNC for user: ${SUDO_USER:-pulsar}..."
VNC_USER="${SUDO_USER:-pulsar}"
VNC_HOME="$(eval echo ~$VNC_USER)"
mkdir -p "$VNC_HOME/.vnc"

openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout "$VNC_HOME/.vnc/self.key" \
  -out "$VNC_HOME/.vnc/self.crt" \
  -subj "/CN=ubuntu" 2>/dev/null
chmod 600 "$VNC_HOME/.vnc/self.key"

cat > "$VNC_HOME/.vnc/kasmvnc.yaml" << 'CONFIG'
network:
  ssl:
    require_ssl: false
    pem_certificate: __CERT__
    pem_key: __KEY__
  websocket_port: 6901
logging:
  log_writer_name: all
  log_dest: logfile
  level: 30
CONFIG
sed -i "s|__CERT__|$VNC_HOME/.vnc/self.crt|g" "$VNC_HOME/.vnc/kasmvnc.yaml"
sed -i "s|__KEY__|$VNC_HOME/.vnc/self.key|g" "$VNC_HOME/.vnc/kasmvnc.yaml"

cat > "$VNC_HOME/.vnc/xstartup" << 'XSTARTUP'
#!/bin/bash
export XDG_SESSION_TYPE=x11
export START_PULSEAUDIO=0
exec startlxqt
XSTARTUP
chmod +x "$VNC_HOME/.vnc/xstartup"

touch "$VNC_HOME/.vnc/.de-was-selected"

expect << EOF
spawn vncpasswd -u $VNC_USER -w $VNC_HOME/.kasmpasswd
expect "Password:"
send "$VNC_PASSWORD\r"
expect "Verify:"
send "$VNC_PASSWORD\r"
expect eof
EOF

chown -R "$VNC_USER:" "$VNC_HOME/.vnc" "$VNC_HOME/.kasmpasswd"

echo "KasmVNC configured for $VNC_USER on display :1 (port 6901)."

# ------------------------------------------------------------------
# 5. Migrate existing user data from kasm_desktop_data/
# ------------------------------------------------------------------
if [ -d "$REPO_DIR/kasm_desktop_data" ] && [ "$(ls -A "$REPO_DIR/kasm_desktop_data" 2>/dev/null)" ]; then
    echo "[5/7] Migrating existing desktop data to $VNC_HOME/..."
    cp -a "$REPO_DIR/kasm_desktop_data/." "$VNC_HOME/"
    echo "Done."
else
    echo "[5/7] No kasm_desktop_data found, skipping migration."
fi

# ------------------------------------------------------------------
# 6. Create systemd service for KasmVNC
# ------------------------------------------------------------------
echo "[6/6] Creating systemd service for KasmVNC..."

cat > /etc/systemd/system/kasmvnc.service << SERVICE
[Unit]
Description=KasmVNC Desktop
After=network.target

[Service]
Type=simple
User=$VNC_USER
ExecStart=/usr/bin/vncserver :1 -geometry 1920x1080 -depth 24 -fg -DisableBasicAuth
ExecStop=/usr/bin/vncserver -kill :1
Restart=on-failure
RestartSec=5
Environment=START_PULSEAUDIO=0

[Install]
WantedBy=multi-user.target
SERVICE

systemctl daemon-reload
systemctl enable kasmvnc

echo ""
echo "=========================================="
echo "  Kasm Desktop Host Setup Complete!"
echo "=========================================="
echo ""
echo "Start services:"
echo "  ./start-services.sh"
echo ""
echo "Connect at: http://<rover-ip>:6901/"
