#!/usr/bin/env bash
# Bootstrap Traefik edge stack (orbytals.com + hermesapp.live) and optionally
# write Mailcow / Technitium compose overrides or patch Forgejo / Synapse configs.
#
# Typical Ubuntu layout: install tree under EDGE_ROOT (default /opt/orbytals/edge).
# Run from a clone of DeployWerk_V2 or set SOURCE_TREE to examples/orbytals-traefik-edge.
#
# Usage:
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh install
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh up
#   sudo MAILCOW_DIR=/opt/mailcow ./scripts/traefik-edge-migrate-orbytals.sh apply-labels
#   ./scripts/traefik-edge-migrate-orbytals.sh verify
#   ./scripts/traefik-edge-migrate-orbytals.sh dns
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh all
#
# Dashboard basic auth (optional): set TRAEFIK_DASHBOARD_USER and TRAEFIK_DASHBOARD_PASSWORD
# before `install` to generate traefik/dynamic/dashboard-auth.yml and enable the middleware
# on the dashboard router.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SOURCE_TREE="${SOURCE_TREE:-${REPO_ROOT}/examples/orbytals-traefik-edge}"
EDGE_ROOT="${EDGE_ROOT:-/opt/orbytals/edge}"
TRAEFIK_PUBLIC_NETWORK="${TRAEFIK_PUBLIC_NETWORK:-proxy}"
MAILCOW_TRAEFIK_NETWORK="${MAILCOW_TRAEFIK_NETWORK:-proxy}"
TECHNITIUM_SERVICE="${TECHNITIUM_SERVICE:-technitium}"
MAILCOW_NGINX_SERVICE="${MAILCOW_NGINX_SERVICE:-nginx-mailcow}"

die() {
  echo "error: $*" >&2
  exit 1
}

require_root_for() {
  if [[ "${EUID}" -ne 0 ]]; then
    die "this command must be run as root: $*"
  fi
}

ensure_traefik_env_kv() {
  local key="$1" val="$2"
  local envf="${EDGE_ROOT}/traefik/.env"
  mkdir -p "$(dirname "$envf")"
  touch "$envf"
  local tmp
  tmp="$(mktemp)"
  grep -v "^${key}=" "$envf" >"$tmp" 2>/dev/null || true
  printf '%s=%s\n' "$key" "$val" >>"$tmp"
  mv "$tmp" "$envf"
}

pick_acme_host_path() {
  if [[ -f /opt/traefik/acme/acme.json ]]; then
    echo "/opt/traefik/acme/acme.json"
    return
  fi
  mkdir -p "${EDGE_ROOT}/traefik/acme"
  local p="${EDGE_ROOT}/traefik/acme/acme.json"
  touch "$p"
  chmod 600 "$p" || true
  echo "$p"
}

copy_tree() {
  [[ -d "$SOURCE_TREE" ]] || die "SOURCE_TREE is not a directory: $SOURCE_TREE"
  mkdir -p "$EDGE_ROOT"
  if command -v rsync >/dev/null 2>&1; then
    rsync -a "${SOURCE_TREE}/" "${EDGE_ROOT}/"
  else
    cp -a "${SOURCE_TREE}/." "${EDGE_ROOT}/"
  fi
}

gen_dashboard_auth() {
  local out="${EDGE_ROOT}/traefik/dynamic/dashboard-auth.yml"
  local dash="${EDGE_ROOT}/traefik/dynamic/dashboard.yml"
  command -v htpasswd >/dev/null 2>&1 || die "htpasswd not found (install apache2-utils)"
  local line
  line="$(htpasswd -nbB "${TRAEFIK_DASHBOARD_USER}" "${TRAEFIK_DASHBOARD_PASSWORD}")"
  local esc=${line//\'/\'\'}
  {
    echo "http:"
    echo "  middlewares:"
    echo "    dashboard-auth:"
    echo "      basicAuth:"
    echo "        removeHeader: true"
    echo "        users:"
    printf "          - '%s'\n" "$esc"
  } >"$out"
  cat >"$dash" <<'EOF'
http:
  routers:
    traefik-dashboard-secure:
      rule: Host(`traefik.orbytals.com`)
      entryPoints:
        - websecure
      tls:
        certResolver: le
      middlewares:
        - dashboard-auth
      service: api@internal
EOF
  echo "Wrote ${out} and updated ${dash} (dashboard basic auth enabled)."
}

cmd_install() {
  require_root_for install
  copy_tree
  local acme_path
  acme_path="$(pick_acme_host_path)"
  chmod 600 "$acme_path" 2>/dev/null || true
  if [[ ! -f "${EDGE_ROOT}/traefik/.env" && -f "${EDGE_ROOT}/traefik/.env.example" ]]; then
    cp "${EDGE_ROOT}/traefik/.env.example" "${EDGE_ROOT}/traefik/.env"
  fi
  ensure_traefik_env_kv ACME_JSON_HOST_PATH "$acme_path"
  docker network create "$TRAEFIK_PUBLIC_NETWORK" >/dev/null 2>&1 || true
  if [[ -n "${TRAEFIK_DASHBOARD_USER:-}" && -n "${TRAEFIK_DASHBOARD_PASSWORD:-}" ]]; then
    gen_dashboard_auth
  else
    echo "Tip: set TRAEFIK_DASHBOARD_USER and TRAEFIK_DASHBOARD_PASSWORD and re-run install to enable dashboard basic auth."
  fi
  echo "Installed edge tree to ${EDGE_ROOT} (from ${SOURCE_TREE})."
}

cmd_stop_legacy() {
  require_root_for stop-legacy
  docker stop traefik >/dev/null 2>&1 || true
  docker rm traefik >/dev/null 2>&1 || true
  echo "Legacy traefik container removed (if it existed)."
}

cmd_up() {
  require_root_for up
  local follow=0
  if [[ "${1:-}" == "--follow-logs" ]]; then
    follow=1
    shift || true
  fi
  [[ -f "${EDGE_ROOT}/traefik/docker-compose.yml" ]] || die "missing ${EDGE_ROOT}/traefik/docker-compose.yml — run install first"
  (cd "${EDGE_ROOT}/traefik" && docker compose up -d "$@")
  if [[ "$follow" -eq 1 ]]; then
    (cd "${EDGE_ROOT}/traefik" && docker compose logs -f traefik)
  fi
}

cmd_dns() {
  cat <<'EOF'
DNS checklist (add at registrar / Cloudflare; not automated from this script):

  TYPE   NAME                  VALUE
  A      orbytals.com          <SERVER_IP>
  A      *.orbytals.com        <SERVER_IP>
  A      hermesapp.live        <SERVER_IP>
  A      *.hermesapp.live      <SERVER_IP>

  MX     orbytals.com          10 mail.orbytals.com
  TXT    orbytals.com          "v=spf1 mx ~all"

  DKIM + DMARC: configure in Mailcow UI after first boot.

Mailcow: set SKIP_LETS_ENCRYPT=y in mailcow.conf (Traefik terminates TLS).
EOF
}

cmd_verify() {
  set +e
  local urls=(
    "https://dns.orbytals.com"
    "https://mail.orbytals.com"
    "https://git.orbytals.com"
    "https://chat.hermesapp.live/_matrix/client/versions"
    "https://chat.hermesapp.live/.well-known/matrix/server"
  )
  local u
  for u in "${urls[@]}"; do
    echo "---- $u"
    curl -fsSI --max-time 15 "$u" || echo "(failed — check DNS / TLS / service labels)"
  done
  set -e
}

write_mailcow_override() {
  local dest="$1"
  cat >"$dest" <<EOF
version: '2.1'
services:
  ${MAILCOW_NGINX_SERVICE}:
    networks:
      traefik_edge: {}
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=${MAILCOW_TRAEFIK_NETWORK}"
      - "traefik.http.routers.mailcow.rule=Host(\`mail.orbytals.com\`)"
      - "traefik.http.routers.mailcow.entrypoints=websecure"
      - "traefik.http.routers.mailcow.tls.certresolver=le"
      - "traefik.http.routers.mailcow.service=mailcow-svc"
      - "traefik.http.routers.mailcow.middlewares=secure-headers@file"
      - "traefik.http.services.mailcow-svc.loadbalancer.server.port=8082"

networks:
  traefik_edge:
    name: ${TRAEFIK_PUBLIC_NETWORK}
    external: true
EOF
  echo "Wrote Mailcow override: $dest"
}

write_technitium_override() {
  local dest="$1"
  cat >"$dest" <<EOF
version: '2.1'
services:
  ${TECHNITIUM_SERVICE}:
    networks:
      traefik_edge: {}
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=${TRAEFIK_PUBLIC_NETWORK}"
      - "traefik.http.routers.technitium.rule=Host(\`dns.orbytals.com\`)"
      - "traefik.http.routers.technitium.entrypoints=websecure"
      - "traefik.http.routers.technitium.tls.certresolver=le"
      - "traefik.http.routers.technitium.service=technitium-svc"
      - "traefik.http.routers.technitium.middlewares=secure-headers@file"
      - "traefik.http.services.technitium-svc.loadbalancer.server.port=5380"

networks:
  traefik_edge:
    name: ${TRAEFIK_PUBLIC_NETWORK}
    external: true
EOF
  echo "Wrote Technitium override: $dest"
}

cmd_apply_labels() {
  require_root_for apply-labels
  if [[ -n "${MAILCOW_DIR:-}" ]]; then
    local mc="${MAILCOW_DIR%/}"
    [[ -d "$mc" ]] || die "MAILCOW_DIR is not a directory: $mc"
    local ov="${mc}/docker-compose.override.yml"
    local side="${mc}/docker-compose.traefik-labels.generated.yml"
    if [[ -f "$ov" ]]; then
      write_mailcow_override "$side"
      echo "docker-compose.override.yml already exists; wrote sidecar: $side"
      echo "  Merge manually or set COMPOSE_FILE=docker-compose.yml:docker-compose.override.yml:docker-compose.traefik-labels.generated.yml"
    else
      write_mailcow_override "$ov"
    fi
    echo "Mailcow: ensure mailcow.conf contains SKIP_LETS_ENCRYPT=y (not modified by this script)."
    grep -q '^SKIP_LETS_ENCRYPT=y' "${mc}/mailcow.conf" 2>/dev/null || echo "  WARN: SKIP_LETS_ENCRYPT=y not detected in ${mc}/mailcow.conf"
  fi

  if [[ -n "${TECHNITIUM_COMPOSE_DIR:-}" ]]; then
    local td="${TECHNITIUM_COMPOSE_DIR%/}"
    [[ -d "$td" ]] || die "TECHNITIUM_COMPOSE_DIR is not a directory: $td"
    local ov="${td}/docker-compose.override.yml"
    local side="${td}/docker-compose.traefik-labels.generated.yml"
    if [[ -f "$ov" ]]; then
      write_technitium_override "$side"
      echo "docker-compose.override.yml already exists; wrote sidecar: $side"
    else
      write_technitium_override "$ov"
    fi
  fi

  if [[ -n "${FORGEJO_APP_INI:-}" ]]; then
    [[ -f "${FORGEJO_APP_INI}" ]] || die "FORGEJO_APP_INI not a file: ${FORGEJO_APP_INI}"
    if grep -q '^ROOT_URL=' "${FORGEJO_APP_INI}"; then
      sed -i.bak 's|^ROOT_URL=.*|ROOT_URL=https://git.orbytals.com/|' "${FORGEJO_APP_INI}" && rm -f "${FORGEJO_APP_INI}.bak"
    else
      printf '\nROOT_URL=https://git.orbytals.com/\n' >>"${FORGEJO_APP_INI}"
    fi
    echo "Updated ROOT_URL in ${FORGEJO_APP_INI} (backup .bak if sed created one)."
  elif [[ -n "${FORGEJO_DATA_DIR:-}" ]]; then
    local fd="${FORGEJO_DATA_DIR%/}"
    local ini="${fd}/custom/conf/app.ini"
    [[ -f "$ini" ]] || die "expected Forgejo app.ini at $ini"
    if grep -q '^ROOT_URL=' "$ini"; then
      sed -i.bak 's|^ROOT_URL=.*|ROOT_URL=https://git.orbytals.com/|' "$ini" && rm -f "${ini}.bak"
    else
      printf '\nROOT_URL=https://git.orbytals.com/\n' >>"$ini"
    fi
    echo "Updated ROOT_URL in $ini"
  fi

  if [[ -n "${SYNAPSE_HOMESERVER_YAML:-}" ]]; then
    local sy="${SYNAPSE_HOMESERVER_YAML}"
    [[ -f "$sy" ]] || die "SYNAPSE_HOMESERVER_YAML not a file: $sy"
    patch_synapse_yaml "$sy"
  elif [[ -n "${SYNAPSE_DATA_DIR:-}" ]]; then
    local sd="${SYNAPSE_DATA_DIR%/}"
    local sy="${sd}/homeserver.yaml"
    if [[ ! -f "$sy" && -f "${sd}/data/homeserver.yaml" ]]; then
      sy="${sd}/data/homeserver.yaml"
    fi
    [[ -f "$sy" ]] || die "homeserver.yaml not found under ${SYNAPSE_DATA_DIR}"
    patch_synapse_yaml "$sy"
  fi
}

patch_synapse_yaml() {
  local sy="$1"
  cp -a "$sy" "${sy}.bak.$(date +%s)" || true
  if grep -qE '^public_baseurl:' "$sy"; then
    sed -i 's|^public_baseurl:.*|public_baseurl: "https://chat.hermesapp.live/"|' "$sy"
  else
    printf '\npublic_baseurl: "https://chat.hermesapp.live/"\n' >>"$sy"
  fi
  if grep -qE '^serve_server_wellknown:' "$sy"; then
    sed -i 's|^serve_server_wellknown:.*|serve_server_wellknown: true|' "$sy"
  else
    printf 'serve_server_wellknown: true\n' >>"$sy"
  fi
  echo "Patched Synapse $(basename "$sy") for public_baseurl and serve_server_wellknown (timestamped .bak backup created)."
}

cmd_all() {
  cmd_install
  cmd_stop_legacy
  cmd_up
  cmd_apply_labels
  cmd_dns
  echo "Running verify (may fail until DNS and all labels are live)..."
  cmd_verify
}

usage() {
  cat <<'EOF'
Commands: install | stop-legacy | up [--follow-logs] | apply-labels | verify | dns | all

Environment:
  SOURCE_TREE   default: <repo>/examples/orbytals-traefik-edge
  EDGE_ROOT     default: /opt/orbytals/edge
  TRAEFIK_PUBLIC_NETWORK   default: proxy
  MAILCOW_TRAEFIK_NETWORK  traefik.docker.network label for Mailcow (default: proxy)
  MAILCOW_NGINX_SERVICE    default: nginx-mailcow
  TECHNITIUM_SERVICE       compose service name (default: technitium)

Optional apply-labels:
  MAILCOW_DIR, TECHNITIUM_COMPOSE_DIR, FORGEJO_APP_INI or FORGEJO_DATA_DIR,
  SYNAPSE_HOMESERVER_YAML or SYNAPSE_DATA_DIR

Dashboard auth on install:
  TRAEFIK_DASHBOARD_USER, TRAEFIK_DASHBOARD_PASSWORD (requires htpasswd)
EOF
}

main() {
  local sub="${1:-}"
  shift || true
  case "$sub" in
    install) cmd_install ;;
    stop-legacy) cmd_stop_legacy ;;
    up) cmd_up "$@" ;;
    apply-labels) cmd_apply_labels ;;
    verify) cmd_verify ;;
    dns) cmd_dns ;;
    all) cmd_all ;;
    "" | -h | --help) usage ;;
    *) die "unknown command: $sub" ;;
  esac
}

main "$@"
