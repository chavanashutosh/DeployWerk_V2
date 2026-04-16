#!/usr/bin/env bash
set -euo pipefail

OPEN_COCKPIT_PORT="${OPEN_COCKPIT_PORT:-false}"
INSTALL_XRDP="${INSTALL_XRDP:-true}"

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root."
  exit 1
fi

echo "== Base packages =="
apt update
apt install -y \
  ca-certificates \
  curl \
  git \
  gnupg \
  jq \
  apache2-utils \
  ufw \
  fail2ban \
  nginx \
  postgresql \
  postgresql-contrib \
  build-essential \
  pkg-config \
  libssl-dev \
  xz-utils \
  cockpit \
  cockpit-pcp

echo "== Node.js 22 =="
if ! command -v node >/dev/null 2>&1 || ! node --version | grep -q '^v22\.'; then
  curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
  apt install -y nodejs
fi

echo "== UFW baseline (edit later as needed) =="
ufw default deny incoming || true
ufw default allow outgoing || true
ufw allow 22/tcp || true
ufw allow 80/tcp || true
ufw allow 443/tcp || true
ufw allow 25/tcp || true
ufw allow 465/tcp || true
ufw allow 587/tcp || true
ufw allow 110/tcp || true
ufw allow 995/tcp || true
ufw allow 143/tcp || true
ufw allow 993/tcp || true
ufw allow 4190/tcp || true
ufw allow 8448/tcp || true
ufw allow 3389/tcp || true
ufw allow 53/tcp || true
ufw allow 53/udp || true
if [[ "${OPEN_COCKPIT_PORT}" == "true" ]]; then
  ufw allow 9090/tcp || true
fi
ufw --force enable || true

echo "== Cockpit =="
systemctl enable --now cockpit.socket

if [[ "${INSTALL_XRDP}" == "true" ]]; then
  echo "== XRDP =="
  apt install -y xrdp
  systemctl enable --now xrdp
else
  echo "== XRDP skipped (INSTALL_XRDP=${INSTALL_XRDP}) =="
fi

echo "== Docker Engine (official convenience script) =="
echo "NOTE: For production, prefer Docker's official apt repo install."
curl -fsSL https://get.docker.com | sh
systemctl enable --now docker

echo "== Traefik base dirs =="
mkdir -p /opt/traefik/acme
touch /opt/traefik/acme/acme.json
chmod 600 /opt/traefik/acme/acme.json

echo "== Create proxy network =="
docker network create proxy >/dev/null 2>&1 || true

echo "== Native DeployWerk dirs =="
mkdir -p /etc/deploywerk /var/www/deploywerk /var/lib/deploywerk/git-cache /var/lib/deploywerk/volumes

echo "== Done =="
echo "Next steps:"
echo "- Full operator guide: README.md (Traefik, Mailcow, Matrix, Technitium, native DeployWerk, env, SSO)."
echo "- Same host as Traefik: bind nginx for SPA/API to 127.0.0.1:8085 (or similar); Traefik terminates TLS."
echo "- Cockpit is installed natively; access via https://<server>:9090 or proxy it through Traefik."
echo "- Example Traefik routes: docs/traefik/orbytals-file-provider.example.yml"
echo "- UFW: this script opens mail, DNS, Matrix federation (8448), and RDP. API stays on 127.0.0.1:8080."
