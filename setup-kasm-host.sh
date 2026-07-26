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
echo "[4/7] Configuring KasmVNC for root..."
mkdir -p /root/.vnc

cat > /root/.vnc/kasmvnc.yaml << 'CONFIG'
network:
  ssl:
    require_ssl: false
  websocket_port: 6901
logging:
  log_writer_name: all
  log_dest: logfile
  level: 30
CONFIG

cat > /root/.vnc/xstartup << 'XSTARTUP'
#!/bin/bash
export XDG_SESSION_TYPE=x11
export START_PULSEAUDIO=0
exec startlxqt
XSTARTUP
chmod +x /root/.vnc/xstartup

touch /root/.vnc/.de-was-selected

expect << EOF
spawn vncpasswd -u root -w /root/.kasmpasswd
expect "Password:"
send "$VNC_PASSWORD\r"
expect "Verify:"
send "$VNC_PASSWORD\r"
expect eof
EOF

echo "KasmVNC configured for root on display :1 (port 6901)."

# ------------------------------------------------------------------
# 5. Migrate existing user data from kasm_desktop_data/
# ------------------------------------------------------------------
if [ -d "$REPO_DIR/kasm_desktop_data" ] && [ "$(ls -A "$REPO_DIR/kasm_desktop_data" 2>/dev/null)" ]; then
    echo "[5/7] Migrating existing desktop data to /root/..."
    cp -a "$REPO_DIR/kasm_desktop_data/." /root/
    echo "Done."
else
    echo "[5/7] No kasm_desktop_data found, skipping migration."
fi

# ------------------------------------------------------------------
# 6. Install and configure Nginx reverse proxy (6900 -> 6901)
# ------------------------------------------------------------------
if ! command -v nginx &>/dev/null; then
    echo "[6/7] Installing Nginx..."
    apt install -y nginx
else
    echo "[6/7] Nginx already installed."
fi

cat > /etc/nginx/sites-available/kasm-desktop << 'NGINX'
map $http_upgrade $connection_upgrade {
    default upgrade;
    '' close;
}

upstream kasm_desktop {
    server localhost:6901;
}

server {
    listen 6900;

    location / {
        proxy_pass http://kasm_desktop;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
NGINX

ln -sf /etc/nginx/sites-available/kasm-desktop /etc/nginx/sites-enabled/
rm -f /etc/nginx/sites-enabled/default

nginx -t && systemctl reload nginx
echo "Nginx configured: port 6900 -> KasmVNC 6901"

# ------------------------------------------------------------------
# 7. Create systemd service for KasmVNC
# ------------------------------------------------------------------
echo "[7/7] Creating systemd service for KasmVNC..."

cat > /etc/systemd/system/kasmvnc.service << 'SERVICE'
[Unit]
Description=KasmVNC Desktop (Root)
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/bin/vncserver :1 -geometry 1920x1080 -depth 24 -fg -DisableBasicAuth
ExecStop=/usr/bin/vncserver -kill :1
Restart=unless-stopped
RestartSec=5
Environment=START_PULSEAUDIO=0

[Install]
WantedBy=multi-user.target
SERVICE

systemctl daemon-reload
systemctl enable kasmvnc
systemctl enable nginx

echo ""
echo "=========================================="
echo "  Kasm Desktop Host Setup Complete!"
echo "=========================================="
echo ""
echo "Start services now:"
echo "  sudo systemctl start kasmvnc"
echo "  sudo systemctl start nginx"
echo ""
echo "Connect at: http://<rover-ip>:6900/"
echo ""
echo "Note: Update your rover's docker-compose.yml"
echo "      by removing the 'desktop' and 'web' services."
echo "      Then run: docker compose up -d"
