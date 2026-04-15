#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root."
  exit 1
fi

echo "== Base packages =="
apt update
apt install -y ca-certificates curl git ufw fail2ban

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
ufw --force enable || true

echo "== XRDP =="
apt install -y xrdp
systemctl enable --now xrdp

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

echo "== Done =="
echo "Next steps:"
echo "- Follow docs/BARE_METAL.md (Host B) to configure Traefik compose, Mailcow, Matrix, Technitium and DNS records."
