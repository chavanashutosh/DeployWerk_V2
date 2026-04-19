#!/usr/bin/env bash
# aaPanel-oriented helper (replaces the former all-in-one Orbytals stack installer).
# Official aaPanel install is interactive; this script only prints instructions or downloads the upstream script.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

AAPANEL_INSTALL_URL="${AAPANEL_INSTALL_URL:-https://www.aapanel.com/script/install_panel_en.sh}"

die() {
  echo "error: $*" >&2
  exit 1
}

require_root() {
  [[ "$(id -u)" -eq 0 ]] || die "run as root (e.g. sudo -i) for aaPanel install steps"
}

usage() {
  cat <<EOF
Usage:
  sudo bash scripts/orbytals-install.sh [command]

Commands:
  help          Show this message
  instructions  Print aaPanel install steps and firewall hints (default)
  download      Fetch the official aaPanel install script to /tmp (does not execute it)

Official documentation:
  https://www.aapanel.com/docs/guide/quickstart.html

The upstream installer is interactive. There is no supported fully non-interactive mode in this repo.
EOF
}

print_instructions() {
  cat <<EOF
================================================================================
aaPanel (recommended control panel) — quick path
================================================================================

Before you start (from aaPanel docs):
  - Supported: Ubuntu 22/24, Debian 11/12/13, CentOS/Alma/Rocky variants (see quickstart).
  - Run as root: sudo -i
  - Minimum: ~512MB RAM, 1GB disk (more for production).

Official one-liner (English panel, copies upstream script and runs it):
  URL=${AAPANEL_INSTALL_URL} && if [ -f /usr/bin/curl ]; then curl -ksSO "\$URL" ; else wget --no-check-certificate -O install_panel_en.sh "\$URL"; fi && bash install_panel_en.sh forum

You will be prompted (e.g. install under /www). When finished, the installer prints:
  - Panel URL (https://YOUR_IP:PANEL_PORT/...)
  - Username and password (or use: bt default / bt 5 on the server)

Firewall / security group — allow at least:
  - 22     TCP   SSH
  - 80     TCP   HTTP (Let's Encrypt HTTP-01, sites)
  - 443    TCP   HTTPS
  - Panel port shown at end of install (example docs mention 31750; yours may differ)
  - Optional: 888, 20, 21 (FTP-related — only if you use those aaPanel features)

See upstream message after install for the exact port list.

================================================================================
DeployWerk after aaPanel (high level)
================================================================================

Repo layout and secrets: README.md → "Where to put the code (Debian 13 production)"
Config template: ${REPO_ROOT}/.env.example → /etc/deploywerk/deploywerk.env (chmod 600)

In aaPanel, typical steps:
  1. Install Nginx, PostgreSQL (and Node or use your own Rust binary from cargo build).
  2. Create database + user for DeployWerk; set DATABASE_URL in deploywerk.env.
  3. Build deploywerk-api (cargo) and install a systemd unit, or run under aaPanel process manager.
  4. Point a website at /var/www/deploywerk (built web/dist) and reverse-proxy /api to 127.0.0.1:8080.
  5. Issue Let's Encrypt for your public hostname in the panel.

Manual Traefik / multi-container examples (no installer here):
  ${REPO_ROOT}/examples/orbytals-traefik-edge/
  ${REPO_ROOT}/docs/traefik/

EOF
}

cmd_download() {
  require_root
  local out="/tmp/install_panel_en.sh"
  echo "Downloading ${AAPANEL_INSTALL_URL} -> ${out}"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${AAPANEL_INSTALL_URL}" -o "${out}"
  else
    wget -q -O "${out}" "${AAPANEL_INSTALL_URL}"
  fi
  chmod 700 "${out}"
  echo "Downloaded. Review the script, then run (as root):"
  echo "  bash ${out} forum"
}

main() {
  local sub="${1:-instructions}"
  case "$sub" in
    help|-h|--help) usage ;;
    instructions) print_instructions ;;
    download) cmd_download ;;
    *) die "unknown command: $sub (try: help)" ;;
  esac
}

main "$@"
