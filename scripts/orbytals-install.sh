#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SOURCE_TREE="${SOURCE_TREE:-${REPO_ROOT}/examples/orbytals-traefik-edge}"

INSTALL_ROOT="${INSTALL_ROOT:-/opt/orbytals}"
EDGE_ROOT="${EDGE_ROOT:-${INSTALL_ROOT}/edge}"
SERVICE_ROOT="${SERVICE_ROOT:-${INSTALL_ROOT}/services}"
STATE_DIR="${STATE_DIR:-/etc/orbytals}"
STATE_FILE="${STATE_FILE:-${STATE_DIR}/install.env}"

TRAEFIK_PUBLIC_NETWORK="${TRAEFIK_PUBLIC_NETWORK:-proxy}"
TRAEFIK_CONTAINER_NAME="${TRAEFIK_CONTAINER_NAME:-traefik}"
TRAEFIK_DASHBOARD_LOCAL_PORT="${TRAEFIK_DASHBOARD_LOCAL_PORT:-18080}"

ORBYTALS_APEX_DOMAIN="${ORBYTALS_APEX_DOMAIN:-orbytals.com}"
ORBYTALS_APP_DOMAIN="${ORBYTALS_APP_DOMAIN:-app.orbytals.com}"
ORBYTALS_API_DOMAIN="${ORBYTALS_API_DOMAIN:-api.orbytals.com}"
ORBYTALS_MAIL_DOMAIN="${ORBYTALS_MAIL_DOMAIN:-mail.orbytals.com}"
ORBYTALS_GIT_DOMAIN="${ORBYTALS_GIT_DOMAIN:-git.orbytals.com}"
ORBYTALS_DNS_DOMAIN="${ORBYTALS_DNS_DOMAIN:-dns.orbytals.com}"
ORBYTALS_TRAEFIK_DOMAIN="${ORBYTALS_TRAEFIK_DOMAIN:-traefik.orbytals.com}"
ORBYTALS_COCKPIT_DOMAIN="${ORBYTALS_COCKPIT_DOMAIN:-cockpit.orbytals.com}"
HERMES_CHAT_DOMAIN="${HERMES_CHAT_DOMAIN:-chat.hermesapp.live}"

DEPLOYWERK_USER="${DEPLOYWERK_USER:-deploywerk}"
DEPLOYWERK_HOME="${DEPLOYWERK_HOME:-/var/lib/deploywerk}"
DEPLOYWERK_STATE_ROOT="${DEPLOYWERK_STATE_ROOT:-/var/lib/deploywerk}"
DEPLOYWERK_ENV_FILE="${DEPLOYWERK_ENV_FILE:-/etc/deploywerk/deploywerk.env}"
DEPLOYWERK_WEB_ROOT="${DEPLOYWERK_WEB_ROOT:-/var/www/deploywerk}"
DEPLOYWERK_API_BIN="${DEPLOYWERK_API_BIN:-/usr/local/bin/deploywerk-api}"
DEPLOYWERK_WORKER_BIN="${DEPLOYWERK_WORKER_BIN:-/usr/local/bin/deploywerk-deploy-worker}"
DEPLOYWERK_API_SERVICE_FILE="${DEPLOYWERK_API_SERVICE_FILE:-/etc/systemd/system/deploywerk-api.service}"
DEPLOYWERK_WORKER_SERVICE_FILE="${DEPLOYWERK_WORKER_SERVICE_FILE:-/etc/systemd/system/deploywerk-deploy-worker.service}"
DEPLOYWERK_NGINX_SITE="${DEPLOYWERK_NGINX_SITE:-/etc/nginx/sites-available/deploywerk.conf}"
DEPLOYWERK_NGINX_ENABLED_SITE="${DEPLOYWERK_NGINX_ENABLED_SITE:-/etc/nginx/sites-enabled/deploywerk.conf}"
DEPLOYWERK_LOOPBACK_HOST="${DEPLOYWERK_LOOPBACK_HOST:-127.0.0.1}"
DEPLOYWERK_API_PORT="${DEPLOYWERK_API_PORT:-8080}"
DEPLOYWERK_NGINX_PORT="${DEPLOYWERK_NGINX_PORT:-8085}"
DEPLOYWERK_DB_NAME="${DEPLOYWERK_DB_NAME:-deploywerk}"
DEPLOYWERK_DB_USER="${DEPLOYWERK_DB_USER:-deploywerk}"
DEPLOYWERK_DB_PASSWORD="${DEPLOYWERK_DB_PASSWORD:-deploywerk}"

MAILCOW_DIR="${MAILCOW_DIR:-/opt/mailcow-dockerized}"
MAILCOW_CLONE_URL="${MAILCOW_CLONE_URL:-https://github.com/mailcow/mailcow-dockerized}"
MAILCOW_BRANCH="${MAILCOW_BRANCH:-master}"
MAILCOW_HTTP_BIND="${MAILCOW_HTTP_BIND:-127.0.0.1}"
MAILCOW_HTTP_PORT="${MAILCOW_HTTP_PORT:-8082}"
MAILCOW_HTTPS_BIND="${MAILCOW_HTTPS_BIND:-127.0.0.1}"
MAILCOW_HTTPS_PORT="${MAILCOW_HTTPS_PORT:-8444}"
MAILCOW_TRAEFIK_OVERRIDE_FILE="${MAILCOW_TRAEFIK_OVERRIDE_FILE:-docker-compose.orbytals-traefik.yml}"

GARAGE_DIR="${GARAGE_DIR:-${SERVICE_ROOT}/garage}"
GARAGE_COMPOSE_FILE="${GARAGE_COMPOSE_FILE:-${GARAGE_DIR}/docker-compose.yml}"
GARAGE_CONFIG_FILE="${GARAGE_CONFIG_FILE:-${GARAGE_DIR}/garage.toml}"
GARAGE_S3_PORT="${GARAGE_S3_PORT:-3900}"
GARAGE_RPC_PORT="${GARAGE_RPC_PORT:-3901}"
GARAGE_WEB_PORT="${GARAGE_WEB_PORT:-3902}"
GARAGE_ADMIN_PORT="${GARAGE_ADMIN_PORT:-3903}"
GARAGE_ENDPOINT_URL="${GARAGE_ENDPOINT_URL:-http://127.0.0.1:${GARAGE_S3_PORT}}"
# Required by current Garage images for [s3_web]; suffix for website-style bucket hosts (Garage configuration reference).
GARAGE_S3_WEB_ROOT_DOMAIN="${GARAGE_S3_WEB_ROOT_DOMAIN:-.web.garage.localhost}"
GARAGE_REGION="${GARAGE_REGION:-garage}"
GARAGE_BUCKET_NAME="${GARAGE_BUCKET_NAME:-deploywerk}"
GARAGE_KEY_NAME="${GARAGE_KEY_NAME:-deploywerk-key}"

FORGEJO_DIR="${FORGEJO_DIR:-${SERVICE_ROOT}/forgejo}"
FORGEJO_COMPOSE_FILE="${FORGEJO_COMPOSE_FILE:-${FORGEJO_DIR}/docker-compose.yml}"
FORGEJO_HTTP_PORT="${FORGEJO_HTTP_PORT:-3000}"
FORGEJO_SSH_PORT="${FORGEJO_SSH_PORT:-2222}"

SYNAPSE_DIR="${SYNAPSE_DIR:-${SERVICE_ROOT}/synapse}"
SYNAPSE_COMPOSE_FILE="${SYNAPSE_COMPOSE_FILE:-${SYNAPSE_DIR}/docker-compose.yml}"
SYNAPSE_CONFIG_DIR="${SYNAPSE_CONFIG_DIR:-${SYNAPSE_DIR}/data}"
SYNAPSE_SERVICE_NAME="${SYNAPSE_SERVICE_NAME:-synapse}"
SYNAPSE_HTTP_PORT="${SYNAPSE_HTTP_PORT:-8008}"

TECHNITIUM_DIR="${TECHNITIUM_DIR:-${SERVICE_ROOT}/technitium}"
TECHNITIUM_COMPOSE_FILE="${TECHNITIUM_COMPOSE_FILE:-${TECHNITIUM_DIR}/docker-compose.yml}"
TECHNITIUM_HTTP_PORT="${TECHNITIUM_HTTP_PORT:-5380}"
TECHNITIUM_DNS_PORT="${TECHNITIUM_DNS_PORT:-8053}"

OPEN_COCKPIT_PORT="${OPEN_COCKPIT_PORT:-false}"
INSTALL_XRDP="${INSTALL_XRDP:-false}"
ENABLE_PUBLIC_MAIL_PORTS="${ENABLE_PUBLIC_MAIL_PORTS:-true}"
ENABLE_PUBLIC_DNS_PORTS="${ENABLE_PUBLIC_DNS_PORTS:-true}"
ENABLE_STANDARD_DNS_PORT_53="${ENABLE_STANDARD_DNS_PORT_53:-false}"
ENABLE_PUBLIC_MATRIX_FEDERATION_PORT="${ENABLE_PUBLIC_MATRIX_FEDERATION_PORT:-true}"
COCKPIT_USE_NETWORKMANAGER="${COCKPIT_USE_NETWORKMANAGER:-true}"
COCKPIT_PORT="${COCKPIT_PORT:-9292}"

die() {
  echo "error: $*" >&2
  exit 1
}

log() {
  echo "== $* =="
}

warn() {
  echo "WARN: $*" >&2
}

require_root() {
  [[ "${EUID}" -eq 0 ]] || die "run as root"
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

ensure_dir() {
  mkdir -p "$@"
}

# Strip leading/trailing ASCII whitespace (state values must not have spaces after "=" when sourced).
trim_outer_ws() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

ensure_env_kv() {
  local file="$1" key="$2" val="$3"
  # Never write raw newlines: a sourced install.env would execute continuation lines as commands.
  val="${val//$'\r'/}"
  val="${val//$'\n'/}"
  val="$(trim_outer_ws "${val}")"
  ensure_dir "$(dirname "$file")"
  touch "$file"
  local tmp
  tmp="$(mktemp)"
  grep -v "^${key}=" "$file" >"$tmp" 2>/dev/null || true
  if [[ "${file}" == "${STATE_FILE}" ]]; then
    printf '%s=%q\n' "$key" "$val" >>"$tmp"
  else
    printf '%s=%s\n' "$key" "$val" >>"$tmp"
  fi
  mv "$tmp" "$file"
}

env_value() {
  local file="$1" key="$2"
  [[ -f "$file" ]] || return 0
  awk -F= -v key="$key" '$1 == key {sub(/^[^=]*=/, "", $0); print $0}' "$file" | tail -n 1
}

ensure_conf_kv() {
  local file="$1" key="$2" val="$3"
  ensure_dir "$(dirname "$file")"
  touch "$file"
  local tmp
  tmp="$(mktemp)"
  grep -v "^${key}=" "$file" >"$tmp" 2>/dev/null || true
  printf '%s=%s\n' "$key" "$val" >>"$tmp"
  mv "$tmp" "$file"
}

generate_jwt_secret() {
  openssl rand -base64 48 | tr -d '\n'
}

generate_hex32() {
  openssl rand -hex 32 | tr -d '\n'
}

generate_alpha_secret() {
  openssl rand -hex 24 | tr -d '\n'
}

# install.env is sourced as bash; orphan lines run as commands. "KEY= value" runs `value` as a command.
# Keep only blank lines, comments, and KEY=value (optional leading "export "); normalize unquoted values.
sanitize_state_file_for_source() {
  [[ -f "${STATE_FILE}" ]] || return 0
  local tmp bad=0
  tmp="$(mktemp)"
  while IFS= read -r line || [[ -n "${line}" ]]; do
    line="${line//$'\r'/}"
    if [[ -z "${line}" ]]; then
      printf '\n' >>"${tmp}"
      continue
    fi
    if [[ "${line}" =~ ^[[:space:]]*# ]]; then
      printf '%s\n' "${line}" >>"${tmp}"
      continue
    fi
    local raw="${line}"
    if [[ "${line}" =~ ^[[:space:]]*export[[:space:]]+(.*)$ ]]; then
      raw="${BASH_REMATCH[1]}"
      raw="${raw#"${raw%%[![:space:]]*}"}"
    fi
    if [[ "${raw}" =~ ^([A-Za-z_][A-Za-z0-9_]*)=(.*)$ ]]; then
      local k="${BASH_REMATCH[1]}" v="${BASH_REMATCH[2]}"
      # Already shell-quoted on the RHS — keep line as-is (do not split/combine with trim+%q).
      if [[ "${v}" == \$\'* || "${v}" == \"* || "${v}" == \'* ]]; then
        printf '%s\n' "${line}" >>"${tmp}"
      else
        printf '%s=%q\n' "${k}" "$(trim_outer_ws "${v}")" >>"${tmp}"
      fi
    else
      bad=$((bad + 1))
    fi
  done <"${STATE_FILE}"
  if cmp -s "${tmp}" "${STATE_FILE}" 2>/dev/null; then
    rm -f "${tmp}"
    return 0
  fi
  if [[ "${bad}" -gt 0 ]]; then
    warn "Removed ${bad} invalid line(s) from ${STATE_FILE} (orphan text or lines without KEY=value). Often caused by a pasted secret splitting across lines; fix values or delete the file and re-run prompts."
  else
    warn "Rewrote ${STATE_FILE} for safe sourcing (e.g. spaces after \"=\" make bash run the next word as a command; values are trimmed and re-quoted where needed)."
  fi
  install -m 600 "${tmp}" "${STATE_FILE}"
  rm -f "${tmp}"
}

load_state() {
  if [[ -f "${STATE_FILE}" ]]; then
    sanitize_state_file_for_source
    # shellcheck disable=SC1090
    source "${STATE_FILE}"
  fi
}

save_state_var() {
  local key="$1" val="$2"
  ensure_env_kv "${STATE_FILE}" "$key" "$val"
  chmod 600 "${STATE_FILE}"
  export "${key}=${val}"
}

prompt_with_default() {
  local var_name="$1" prompt="$2" default="${3:-}"
  local current="${!var_name:-$default}"
  if [[ -n "$current" ]]; then
    printf "%s [%s]: " "$prompt" "$current" >&2
  else
    printf "%s: " "$prompt" >&2
  fi
  local answer
  IFS= read -r answer || true
  if [[ -z "$answer" ]]; then
    answer="$current"
  fi
  [[ -n "$answer" ]] || die "$var_name is required"
  save_state_var "$var_name" "$answer"
}

prompt_secret() {
  local var_name="$1" prompt="$2"
  if [[ -n "${!var_name:-}" ]]; then
    save_state_var "$var_name" "${!var_name}"
    return
  fi
  [[ -t 0 ]] || die "$var_name is missing and no TTY is available"
  local first second
  while true; do
    read -r -s -p "${prompt}: " first
    echo >&2
    read -r -s -p "Confirm ${prompt}: " second
    echo >&2
    [[ -n "$first" ]] || { echo "Value cannot be empty." >&2; continue; }
    [[ "$first" == "$second" ]] || { echo "Values do not match." >&2; continue; }
    save_state_var "$var_name" "$first"
    break
  done
}

maybe_generate_state_var() {
  local key="$1" generator="$2"
  if [[ -z "${!key:-}" ]]; then
    save_state_var "$key" "$($generator)"
  fi
}

collect_inputs() {
  load_state
  prompt_with_default ADMIN_USERNAME "Operator username" "ashadmin"
  prompt_secret ADMIN_PASSWORD "Operator password"
  prompt_with_default ACME_EMAIL "Traefik ACME email" "postmaster@${ORBYTALS_APEX_DOMAIN}"
  prompt_with_default FORGEJO_ADMIN_EMAIL "Forgejo admin email" "${ADMIN_USERNAME}@${ORBYTALS_APEX_DOMAIN}"
  prompt_with_default MAILCOW_TIMEZONE "Mailcow timezone" "UTC"
  prompt_with_default DEPLOYWERK_BOOTSTRAP_PLATFORM_ADMIN_EMAIL "DeployWerk bootstrap admin email" "${ADMIN_USERNAME}@${ORBYTALS_APEX_DOMAIN}"
  prompt_with_default DEPLOYWERK_SMTP_USER_PROMPT "DeployWerk SMTP mailbox" "deploywerk@${ORBYTALS_APEX_DOMAIN}"
  prompt_secret DEPLOYWERK_SMTP_PASSWORD_PROMPT "DeployWerk SMTP mailbox password"
  maybe_generate_state_var DEPLOYWERK_DB_PASSWORD generate_alpha_secret
  # Garage rpc_secret: exactly 64 hex chars (32 bytes); must not use generate_alpha_secret (48 hex chars).
  maybe_generate_state_var GARAGE_RPC_SECRET generate_hex32
  maybe_generate_state_var GARAGE_ADMIN_TOKEN generate_alpha_secret
  maybe_generate_state_var MATRIX_REGISTRATION_SHARED_SECRET generate_alpha_secret
  maybe_generate_state_var TECHNITIUM_ADMIN_PASSWORD generate_alpha_secret
  maybe_generate_state_var DEPLOYWERK_JWT_SECRET generate_jwt_secret
  maybe_generate_state_var DEPLOYWERK_SERVER_KEY_ENCRYPTION_KEY generate_hex32
}

ensure_node22() {
  if command_exists node && node --version | grep -q '^v22\.'; then
    return
  fi
  log "Installing Node.js 22"
  curl -fsSL https://deb.nodesource.com/setup_22.x | bash -
  apt install -y nodejs
  command_exists node || die "node install failed"
  node --version | grep -q '^v22\.' || die "Node.js 22 is required; current version is $(node --version)"
}

ensure_root_rustup() {
  export CARGO_HOME="/root/.cargo"
  export RUSTUP_HOME="/root/.rustup"
  if [[ ! -x "${CARGO_HOME}/bin/cargo" ]]; then
    curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
  fi
  # shellcheck disable=SC1091
  source "${CARGO_HOME}/env"
}

configure_firewall() {
  log "Configuring firewall"
  ufw default deny incoming || true
  ufw default allow outgoing || true
  ufw allow 22/tcp || true
  ufw allow 80/tcp || true
  ufw allow 443/tcp || true

  if [[ "${ENABLE_PUBLIC_MAIL_PORTS}" == "true" ]]; then
    for rule in 25/tcp 465/tcp 587/tcp 110/tcp 995/tcp 143/tcp 993/tcp 4190/tcp; do
      ufw allow "$rule" || true
    done
  fi

  if [[ "${ENABLE_PUBLIC_DNS_PORTS}" == "true" ]]; then
    ufw allow "${TECHNITIUM_DNS_PORT}/tcp" || true
    ufw allow "${TECHNITIUM_DNS_PORT}/udp" || true
  fi

  if [[ "${ENABLE_STANDARD_DNS_PORT_53}" == "true" ]]; then
    ufw allow 53/tcp || true
    ufw allow 53/udp || true
  fi

  if [[ "${ENABLE_PUBLIC_MATRIX_FEDERATION_PORT}" == "true" ]]; then
    ufw allow 8448/tcp || true
  fi

  if [[ "${OPEN_COCKPIT_PORT}" == "true" ]]; then
    ufw allow "${COCKPIT_PORT}/tcp" || true
  else
    ufw deny "${COCKPIT_PORT}/tcp" >/dev/null 2>&1 || true
  fi

  ufw --force enable || true
}

configure_cockpit_networkmanager() {
  if [[ "${COCKPIT_USE_NETWORKMANAGER}" != "true" ]]; then
    return
  fi

  log "Configuring NetworkManager for Cockpit updates"
  ensure_dir /etc/NetworkManager/conf.d
  cat >/etc/NetworkManager/conf.d/10-orbytals-managed-devices.conf <<'EOF'
[keyfile]
unmanaged-devices=none
EOF

  if command_exists netplan; then
    cat >/etc/netplan/99-orbytals-networkmanager.yaml <<'EOF'
network:
  version: 2
  renderer: NetworkManager
EOF
    chmod 600 /etc/netplan/99-orbytals-networkmanager.yaml
    netplan generate
  fi

  systemctl enable --now NetworkManager
  systemctl restart packagekit || true
}

configure_cockpit() {
  log "Configuring Cockpit"
  configure_cockpit_socket_port
  systemctl enable --now cockpit.socket
  systemctl enable --now packagekit || true
  configure_cockpit_networkmanager
}

configure_cockpit_socket_port() {
  log "Configuring Cockpit socket port (${COCKPIT_PORT})"
  ensure_dir /etc/systemd/system/cockpit.socket.d
  cat >/etc/systemd/system/cockpit.socket.d/99-orbytals-listen.conf <<EOF
[Socket]
ListenStream=
ListenStream=${COCKPIT_PORT}
EOF
  systemctl daemon-reload
  systemctl restart cockpit.socket || true
}

port_listeners() {
  local port="$1"
  ss -ltnp "sport = :${port}" 2>/dev/null | awk 'NR>1 {print $0}' || true
}

udp_port_listeners() {
  local port="$1"
  ss -lunp "sport = :${port}" 2>/dev/null | awk 'NR>1 {print $0}' || true
}

assert_port_available_or_managed() {
  local port="$1" why="$2"
  local listeners
  listeners="$(port_listeners "$port")"
  if [[ -z "$listeners" ]]; then
    return
  fi
  # If a rerun and the port is held by known managed processes, allow.
  # Otherwise, fail with diagnostics.
  if printf '%s' "$listeners" | grep -Eq '(docker-proxy|traefik|deploywerk-api|nginx|garage|forgejo|technitium|synapse|postfix|dovecot|portainer)'; then
    warn "Port ${port} is already in use (${why}); looks like managed services. Continuing."
    return
  fi
  echo "Port ${port} is already in use (${why})." >&2
  echo "$listeners" >&2
  die "port conflict on ${port}"
}

assert_udp_port_available_or_managed() {
  local port="$1" why="$2"
  local listeners
  listeners="$(udp_port_listeners "$port")"
  if [[ -z "${listeners}" ]]; then
    return
  fi
  if printf '%s' "$listeners" | grep -Eq '(docker-proxy|traefik|deploywerk-api|nginx|garage|forgejo|technitium|synapse|postfix|dovecot|portainer)'; then
    warn "UDP port ${port} is already in use (${why}); looks like managed services. Continuing."
    return
  fi
  echo "UDP port ${port} is already in use (${why})." >&2
  echo "$listeners" >&2
  die "port conflict on udp/${port}"
}

# Cockpit often listens on COCKPIT_PORT via systemd socket activation (ss shows users:(("systemd",pid=1,...))).
assert_cockpit_port_preflight() {
  local port="${COCKPIT_PORT}" why="Cockpit host socket"
  local listeners
  listeners="$(port_listeners "$port")"
  if [[ -z "$listeners" ]]; then
    return
  fi
  if printf '%s' "$listeners" | grep -Eq '(docker-proxy|traefik|deploywerk-api|nginx|garage|forgejo|technitium|synapse|postfix|dovecot|portainer)'; then
    warn "Port ${port} is already in use (${why}); looks like managed services. Continuing."
    return
  fi
  if printf '%s' "$listeners" | grep -Eiq 'cockpit'; then
    warn "Port ${port} is already in use (${why}); Cockpit is active. Continuing."
    return
  fi
  if printf '%s' "$listeners" | grep -q 'systemd'; then
    warn "Port ${port} is already in use (${why}); likely systemd socket activation (Cockpit). Continuing."
    return
  fi
  echo "Port ${port} is already in use (${why})." >&2
  echo "$listeners" >&2
  die "port conflict on ${port}"
}

preflight_ports() {
  log "Preflight: checking for port conflicts"
  # Public ports
  assert_port_available_or_managed 80 "Traefik HTTP"
  assert_port_available_or_managed 443 "Traefik HTTPS"
  if [[ "${ENABLE_PUBLIC_MAIL_PORTS}" == "true" ]]; then
    for p in 25 465 587 110 995 143 993 4190; do
      assert_port_available_or_managed "$p" "Mail port"
    done
  fi
  if [[ "${ENABLE_PUBLIC_DNS_PORTS}" == "true" ]]; then
    assert_port_available_or_managed "${TECHNITIUM_DNS_PORT}" "Technitium DNS (TCP)"
    assert_udp_port_available_or_managed "${TECHNITIUM_DNS_PORT}" "Technitium DNS (UDP)"
  fi
  if [[ "${ENABLE_STANDARD_DNS_PORT_53}" == "true" ]]; then
    assert_port_available_or_managed 53 "Standard DNS (TCP)"
    assert_udp_port_available_or_managed 53 "Standard DNS (UDP)"
  fi
  if [[ "${ENABLE_PUBLIC_MATRIX_FEDERATION_PORT}" == "true" ]]; then
    assert_port_available_or_managed 8448 "Matrix federation"
  fi
  assert_port_available_or_managed "${FORGEJO_SSH_PORT}" "Forgejo SSH"

  # Loopback-only ports
  assert_port_available_or_managed "${TRAEFIK_DASHBOARD_LOCAL_PORT}" "Traefik local dashboard"
  assert_port_available_or_managed "${DEPLOYWERK_API_PORT}" "DeployWerk API"
  assert_port_available_or_managed "${DEPLOYWERK_NGINX_PORT}" "DeployWerk nginx"
  assert_port_available_or_managed "${MAILCOW_HTTP_PORT}" "Mailcow HTTP bind"
  assert_port_available_or_managed "${MAILCOW_HTTPS_PORT}" "Mailcow HTTPS bind"
  assert_cockpit_port_preflight
  assert_port_available_or_managed "${GARAGE_S3_PORT}" "Garage S3"
  assert_port_available_or_managed "${GARAGE_WEB_PORT}" "Garage web"
  assert_port_available_or_managed "${GARAGE_ADMIN_PORT}" "Garage admin"
  assert_port_available_or_managed "${TECHNITIUM_HTTP_PORT}" "Technitium UI"
}

bootstrap_host() {
  log "Installing host packages"
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
    cockpit-networkmanager \
    cockpit-packagekit \
    cockpit-pcp \
    cockpit-storaged \
    network-manager \
    packagekit \
    packagekit-tools \
    udisks2-btrfs \
    udisks2-lvm2 \
    sqlite3

  if apt-cache show udisks2-iscsi >/dev/null 2>&1; then
    apt install -y udisks2-iscsi
  else
    log "Skipping optional package udisks2-iscsi (not available in current apt sources)"
  fi

  ensure_node22
  ensure_root_rustup

  configure_cockpit
  if [[ "${INSTALL_XRDP}" == "true" ]]; then
    apt install -y xrdp
    systemctl enable --now xrdp
  fi

  log "Installing Docker Engine"
  if ! command_exists docker; then
    curl -fsSL https://get.docker.com | sh
  fi
  systemctl enable --now docker

  configure_firewall

  log "Creating shared directories"
  ensure_dir /opt/traefik/acme
  touch /opt/traefik/acme/acme.json
  chmod 600 /opt/traefik/acme/acme.json || true
  ensure_dir "${INSTALL_ROOT}" "${SERVICE_ROOT}" /etc/deploywerk "${DEPLOYWERK_WEB_ROOT}" "${DEPLOYWERK_STATE_ROOT}/git-cache" "${DEPLOYWERK_STATE_ROOT}/volumes"

  docker network create "${TRAEFIK_PUBLIC_NETWORK}" >/dev/null 2>&1 || true
}

disable_default_nginx_site() {
  rm -f /etc/nginx/sites-enabled/default
}

host_gateway_ip() {
  local gw
  gw="$(docker network inspect bridge --format '{{(index .IPAM.Config 0).Gateway}}' 2>/dev/null || true)"
  if [[ -n "$gw" ]]; then
    echo "$gw"
    return
  fi
  ip route | awk '/default/ {print $3; exit}'
}

copy_edge_tree() {
  [[ -d "$SOURCE_TREE" ]] || die "SOURCE_TREE is not a directory: $SOURCE_TREE"
  ensure_dir "$EDGE_ROOT"
  if command_exists rsync; then
    rsync -a "${SOURCE_TREE}/" "${EDGE_ROOT}/"
  else
    cp -a "${SOURCE_TREE}/." "${EDGE_ROOT}/"
  fi
}

write_traefik_native_services() {
  local host_gw
  host_gw="$(host_gateway_ip)"
  cat >"${EDGE_ROOT}/traefik/dynamic/native-services.yml" <<EOF
http:
  routers:
    orbytals-api:
      rule: Host(\`${ORBYTALS_API_DOMAIN}\`)
      entryPoints:
        - websecure
      tls:
        certResolver: le
      service: orbytals-deploywerk-nginx

    orbytals-app:
      rule: Host(\`${ORBYTALS_APP_DOMAIN}\`) || Host(\`${ORBYTALS_APEX_DOMAIN}\`)
      entryPoints:
        - websecure
      tls:
        certResolver: le
      service: orbytals-deploywerk-nginx

    orbytals-cockpit:
      rule: Host(\`${ORBYTALS_COCKPIT_DOMAIN}\`)
      entryPoints:
        - websecure
      tls:
        certResolver: le
      service: orbytals-cockpit

  services:
    orbytals-deploywerk-nginx:
      loadBalancer:
        servers:
          - url: "http://${host_gw}:${DEPLOYWERK_NGINX_PORT}"

    orbytals-cockpit:
      loadBalancer:
        serversTransport: cockpit-insecure-transport
        servers:
          - url: "https://${host_gw}:${COCKPIT_PORT}"

  serversTransports:
    cockpit-insecure-transport:
      insecureSkipVerify: true
EOF
}

gen_dashboard_auth() {
  local out="${EDGE_ROOT}/traefik/dynamic/dashboard-auth.yml"
  local dash="${EDGE_ROOT}/traefik/dynamic/dashboard.yml"
  local line esc
  line="$(htpasswd -nbB "${ADMIN_USERNAME}" "${ADMIN_PASSWORD}")"
  esc=${line//\'/\'\'}
  cat >"$out" <<EOF
http:
  middlewares:
    dashboard-auth:
      basicAuth:
        removeHeader: true
        users:
          - '$esc'
EOF
  cat >"$dash" <<EOF
http:
  routers:
    traefik-dashboard-secure:
      rule: Host(\`${ORBYTALS_TRAEFIK_DOMAIN}\`)
      entryPoints:
        - websecure
      tls:
        certResolver: le
      middlewares:
        - dashboard-auth
      service: api@internal
EOF
}

ensure_traefik_env() {
  local envf="${EDGE_ROOT}/traefik/.env"
  [[ -f "$envf" ]] || cp "${EDGE_ROOT}/traefik/.env.example" "$envf"
  ensure_env_kv "$envf" ACME_EMAIL "${ACME_EMAIL}"
  ensure_env_kv "$envf" ACME_JSON_HOST_PATH "/opt/traefik/acme/acme.json"
}

install_traefik() {
  log "Installing Traefik edge"
  copy_edge_tree
  ensure_traefik_env
  write_traefik_native_services
  gen_dashboard_auth
  (cd "${EDGE_ROOT}/traefik" && docker compose up -d) || {
    (cd "${EDGE_ROOT}/traefik" && docker compose ps) || true
    docker logs "${TRAEFIK_CONTAINER_NAME}" --tail 200 || true
    die "Traefik failed to start"
  }
}

ensure_deploywerk_user() {
  if ! id -u "${DEPLOYWERK_USER}" >/dev/null 2>&1; then
    useradd --system --create-home --home-dir "${DEPLOYWERK_HOME}" --shell /usr/sbin/nologin "${DEPLOYWERK_USER}"
  fi
  ensure_dir "${DEPLOYWERK_STATE_ROOT}/git-cache" "${DEPLOYWERK_STATE_ROOT}/volumes" "${DEPLOYWERK_WEB_ROOT}" /etc/deploywerk
  chown -R "${DEPLOYWERK_USER}:${DEPLOYWERK_USER}" "${DEPLOYWERK_STATE_ROOT}"
}

ensure_postgres_db() {
  systemctl enable --now postgresql
  su - postgres -s /bin/sh -c "psql -tAc \"SELECT 1 FROM pg_roles WHERE rolname='${DEPLOYWERK_DB_USER}'\" | grep -q 1" \
    || su - postgres -s /bin/sh -c "psql -c \"CREATE USER \\\"${DEPLOYWERK_DB_USER}\\\" WITH PASSWORD '${DEPLOYWERK_DB_PASSWORD}';\""
  su - postgres -s /bin/sh -c "psql -tAc \"SELECT 1 FROM pg_database WHERE datname='${DEPLOYWERK_DB_NAME}'\" | grep -q 1" \
    || su - postgres -s /bin/sh -c "createdb -O \"${DEPLOYWERK_DB_USER}\" \"${DEPLOYWERK_DB_NAME}\""
}

ensure_deploywerk_env() {
  [[ -f "${DEPLOYWERK_ENV_FILE}" ]] || cp "${REPO_ROOT}/.env.example" "${DEPLOYWERK_ENV_FILE}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" APP_ENV production
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" HOST "${DEPLOYWERK_LOOPBACK_HOST}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" PORT "${DEPLOYWERK_API_PORT}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DATABASE_URL "postgresql://${DEPLOYWERK_DB_USER}:${DEPLOYWERK_DB_PASSWORD}@127.0.0.1:5432/${DEPLOYWERK_DB_NAME}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" JWT_SECRET "${DEPLOYWERK_JWT_SECRET}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" SERVER_KEY_ENCRYPTION_KEY "${DEPLOYWERK_SERVER_KEY_ENCRYPTION_KEY}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_API_URL "https://${ORBYTALS_API_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_PUBLIC_APP_URL "https://${ORBYTALS_APP_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" VITE_API_URL ""
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_API_PROXY "http://${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_API_PORT}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_GIT_SHA "orbytals-native"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_GIT_CACHE_ROOT "${DEPLOYWERK_STATE_ROOT}/git-cache"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_VOLUMES_ROOT "${DEPLOYWERK_STATE_ROOT}/volumes"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_ENDPOINT_URL "${GARAGE_ENDPOINT_URL}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_BUCKET "${GARAGE_BUCKET_NAME}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_REGION "${GARAGE_REGION}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_PATH_STYLE "true"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_ACCESS_KEY "${GARAGE_ACCESS_KEY_ID}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_SECRET_KEY "${GARAGE_SECRET_ACCESS_KEY}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_PLATFORM_DOCKER_ENABLED "true"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_APPS_BASE_DOMAIN "${ORBYTALS_APEX_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_EDGE_MODE "traefik"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_TRAEFIK_DOCKER_NETWORK "${TRAEFIK_PUBLIC_NETWORK}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_LOCAL_SERVICE_DEFAULTS "false"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_TRAEFIK_URL "https://${ORBYTALS_TRAEFIK_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_FORGEJO_URL "https://${ORBYTALS_GIT_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_TECHNITIUM_URL "https://${ORBYTALS_DNS_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_MAILCOW_URL "https://${ORBYTALS_MAIL_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_MATRIX_CLIENT_URL "https://${HERMES_CHAT_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_BOOTSTRAP_PLATFORM_ADMIN_EMAIL "${DEPLOYWERK_BOOTSTRAP_PLATFORM_ADMIN_EMAIL}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_HOST "127.0.0.1"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_PORT "587"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_USER "${DEPLOYWERK_SMTP_USER_PROMPT}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_PASSWORD "${DEPLOYWERK_SMTP_PASSWORD_PROMPT}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_FROM "DeployWerk <${DEPLOYWERK_SMTP_USER_PROMPT}>"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_TLS "starttls"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_MAIL_ENABLED "true"
  chmod 600 "${DEPLOYWERK_ENV_FILE}"
}

write_deploywerk_api_service() {
  cat >"${DEPLOYWERK_API_SERVICE_FILE}" <<EOF
[Unit]
Description=DeployWerk API
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=simple
User=${DEPLOYWERK_USER}
Group=${DEPLOYWERK_USER}
WorkingDirectory=${DEPLOYWERK_STATE_ROOT}
EnvironmentFile=${DEPLOYWERK_ENV_FILE}
ExecStart=${DEPLOYWERK_API_BIN}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
}

write_deploywerk_worker_service() {
  cat >"${DEPLOYWERK_WORKER_SERVICE_FILE}" <<EOF
[Unit]
Description=DeployWerk Deploy Worker
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=simple
User=${DEPLOYWERK_USER}
Group=${DEPLOYWERK_USER}
WorkingDirectory=${DEPLOYWERK_STATE_ROOT}
EnvironmentFile=${DEPLOYWERK_ENV_FILE}
ExecStart=${DEPLOYWERK_WORKER_BIN}
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
}

write_nginx_site() {
  cat >"${DEPLOYWERK_NGINX_SITE}" <<EOF
server {
  listen ${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_NGINX_PORT};
  server_name ${ORBYTALS_APP_DOMAIN} ${ORBYTALS_API_DOMAIN} ${ORBYTALS_APEX_DOMAIN};
  root ${DEPLOYWERK_WEB_ROOT};
  index index.html;

  location /api/ {
    proxy_pass http://${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_API_PORT};
    proxy_http_version 1.1;
    proxy_set_header Host \$host;
    proxy_set_header X-Forwarded-Host \$host;
    proxy_set_header X-Forwarded-Proto https;
    proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    proxy_set_header X-Real-IP \$remote_addr;
  }

  location / {
    try_files \$uri \$uri/ /index.html;
  }
}
EOF
  ln -snf "${DEPLOYWERK_NGINX_SITE}" "${DEPLOYWERK_NGINX_ENABLED_SITE}"
}

build_native_deploywerk() {
  ensure_root_rustup
  ensure_node22
  (cd "${REPO_ROOT}" && cargo build --release -p deploywerk-api --bin deploywerk-api --bin deploywerk-deploy-worker)
  install -m 0755 "${REPO_ROOT}/target/release/deploywerk-api" "${DEPLOYWERK_API_BIN}"
  install -m 0755 "${REPO_ROOT}/target/release/deploywerk-deploy-worker" "${DEPLOYWERK_WORKER_BIN}"
  (cd "${REPO_ROOT}/web" && npm ci && npm run build)
  ensure_dir "${DEPLOYWERK_WEB_ROOT}"
  cp -a "${REPO_ROOT}/web/dist/." "${DEPLOYWERK_WEB_ROOT}/"
  chown -R "${DEPLOYWERK_USER}:${DEPLOYWERK_USER}" "${DEPLOYWERK_STATE_ROOT}" "${DEPLOYWERK_WEB_ROOT}"
}

install_native_deploywerk() {
  log "Installing native DeployWerk"
  ensure_deploywerk_user
  ensure_postgres_db
  ensure_deploywerk_env
  build_native_deploywerk
  write_deploywerk_api_service
  write_deploywerk_worker_service
  write_nginx_site
  disable_default_nginx_site
  nginx -t
  systemctl daemon-reload
  systemctl enable --now nginx
  systemctl enable --now deploywerk-api
  systemctl disable --now deploywerk-deploy-worker >/dev/null 2>&1 || true
  systemctl restart nginx
}

write_garage_config() {
  ensure_dir "${GARAGE_DIR}" "${GARAGE_DIR}/meta" "${GARAGE_DIR}/data"
  cat >"${GARAGE_CONFIG_FILE}" <<EOF
metadata_dir = "/var/lib/garage/meta"
data_dir = "/var/lib/garage/data"
db_engine = "sqlite"

replication_factor = 1
rpc_bind_addr = "0.0.0.0:${GARAGE_RPC_PORT}"
rpc_public_addr = "127.0.0.1:${GARAGE_RPC_PORT}"
rpc_secret = "${GARAGE_RPC_SECRET}"

[s3_api]
s3_region = "${GARAGE_REGION}"
api_bind_addr = "0.0.0.0:${GARAGE_S3_PORT}"

[s3_web]
bind_addr = "0.0.0.0:${GARAGE_WEB_PORT}"
root_domain = "${GARAGE_S3_WEB_ROOT_DOMAIN}"

[admin]
api_bind_addr = "0.0.0.0:${GARAGE_ADMIN_PORT}"
admin_token = "${GARAGE_ADMIN_TOKEN}"
metrics_token = "${GARAGE_ADMIN_TOKEN}"
EOF
}

write_garage_compose() {
  ensure_dir "${GARAGE_DIR}"
  cat >"${GARAGE_COMPOSE_FILE}" <<EOF
services:
  garage:
    image: dxflrs/garage:v2.1.0
    container_name: garage
    restart: unless-stopped
    ports:
      - "127.0.0.1:${GARAGE_S3_PORT}:${GARAGE_S3_PORT}"
      - "127.0.0.1:${GARAGE_WEB_PORT}:${GARAGE_WEB_PORT}"
      - "127.0.0.1:${GARAGE_ADMIN_PORT}:${GARAGE_ADMIN_PORT}"
    volumes:
      - ${GARAGE_CONFIG_FILE}:/etc/garage.toml
      - ${GARAGE_DIR}/meta:/var/lib/garage/meta
      - ${GARAGE_DIR}/data:/var/lib/garage/data
EOF
}

ensure_garage_rpc_secret_format() {
  if [[ "${GARAGE_RPC_SECRET:-}" =~ ^[0-9a-fA-F]{64}$ ]]; then
    return
  fi
  warn "GARAGE_RPC_SECRET must be 64 hex characters (32 bytes) for Garage; regenerating and updating ${STATE_FILE}."
  save_state_var GARAGE_RPC_SECRET "$(generate_hex32)"
}

install_garage() {
  log "Installing Garage"
  ensure_garage_rpc_secret_format
  write_garage_config
  write_garage_compose
  (cd "${GARAGE_DIR}" && docker compose up -d) || {
    (cd "${GARAGE_DIR}" && docker compose ps) || true
    docker logs garage --tail 200 || true
    die "Garage failed to start"
  }
}

wait_for_container_running() {
  local name="$1" timeout_s="${2:-60}"
  local start
  start="$(date +%s)"
  while true; do
    if docker inspect -f '{{.State.Running}}' "$name" 2>/dev/null | grep -q true; then
      return 0
    fi
    if [[ $(( $(date +%s) - start )) -ge "$timeout_s" ]]; then
      return 1
    fi
    sleep 2
  done
}

compose_up_or_die() {
  local dir="$1" name="$2" container_name="${3:-}"
  (cd "${dir}" && docker compose up -d) || {
    echo "Service ${name} failed to start." >&2
    (cd "${dir}" && docker compose ps) >&2 || true
    if [[ -n "${container_name}" ]]; then
      docker logs "${container_name}" --tail 200 >&2 || true
    fi
    die "${name} failed to start"
  }
}

wait_for_garage_ready() {
  local timeout_s="${1:-120}"
  local start
  start="$(date +%s)"
  while true; do
    if docker inspect -f '{{.State.Running}}' garage 2>/dev/null | grep -q true; then
      if docker exec garage /garage -c /etc/garage.toml status >/dev/null 2>&1; then
        return 0
      fi
    fi
    if [[ $(( $(date +%s) - start )) -ge "$timeout_s" ]]; then
      return 1
    fi
    sleep 2
  done
}

garage_cli() {
  # Retry on transient docker exec errors (including 409) during startup.
  local attempt=1 max_attempts=20 sleep_s=2
  while true; do
    if docker exec garage /garage -c /etc/garage.toml "$@" ; then
      return 0
    fi
    if [[ "${attempt}" -ge "${max_attempts}" ]]; then
      return 1
    fi
    attempt=$((attempt + 1))
    sleep "${sleep_s}"
  done
}

bootstrap_garage() {
  log "Bootstrapping Garage bucket and keys"
  if ! wait_for_garage_ready 180; then
    docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}' | sed -n '1,20p' >&2 || true
    docker logs garage --tail 250 >&2 || true
    die "Garage did not become ready in time; cannot bootstrap"
  fi

  local status_out node_id layout_state key_output key_id secret_key
  status_out="$(garage_cli status 2>/dev/null || true)"
  node_id="$(printf '%s\n' "${status_out}" | awk '/^[0-9a-f]{8,}/ {print $1; exit}')"
  [[ -n "${node_id}" ]] || die "could not determine Garage node id"

  layout_state="$(garage_cli layout show 2>/dev/null || true)"
  if ! printf '%s' "${layout_state}" | grep -q "${node_id}"; then
    garage_cli layout assign -z dc1 -c 20G "${node_id}" >/dev/null
  fi
  # Apply layout if not already applied (safe on reruns).
  if ! printf '%s\n' "${layout_state}" | grep -qi "applied"; then
    garage_cli layout apply --version 1 >/dev/null 2>&1 || true
  fi

  key_output="$(garage_cli key info "${GARAGE_KEY_NAME}" 2>/dev/null || true)"
  if [[ -z "${key_output}" ]]; then
    key_output="$(garage_cli key create "${GARAGE_KEY_NAME}" 2>/dev/null || true)"
  fi
  key_id="$(printf '%s\n' "${key_output}" | awk -F': ' '/Key ID/ {print $2; exit}' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  secret_key="$(printf '%s\n' "${key_output}" | awk -F': ' '/Secret key/ {print $2; exit}' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  if [[ -z "${key_id}" || -z "${secret_key}" ]]; then
    docker logs garage --tail 250 >&2 || true
    die "could not extract Garage S3 credentials"
  fi
  save_state_var GARAGE_ACCESS_KEY_ID "${key_id}"
  save_state_var GARAGE_SECRET_ACCESS_KEY "${secret_key}"

  if ! garage_cli bucket create "${GARAGE_BUCKET_NAME}" >/dev/null 2>&1; then
    # Assume bucket exists on reruns; verify quickly.
    if ! garage_cli bucket info "${GARAGE_BUCKET_NAME}" >/dev/null 2>&1; then
      docker logs garage --tail 250 >&2 || true
      die "Garage bucket create failed"
    fi
  fi

  if ! garage_cli bucket allow --read --write --owner "${GARAGE_BUCKET_NAME}" --key "${GARAGE_KEY_NAME}" >/dev/null 2>&1; then
    docker logs garage --tail 250 >&2 || true
    die "Garage bucket allow failed"
  fi
}

write_technitium_compose() {
  ensure_dir "${TECHNITIUM_DIR}"
  cat >"${TECHNITIUM_COMPOSE_FILE}" <<EOF
services:
  technitium:
    image: technitium/dns-server:latest
    container_name: technitium
    restart: unless-stopped
    environment:
      DNS_SERVER_DOMAIN: ${ORBYTALS_DNS_DOMAIN}
      DNS_SERVER_ADMIN_PASSWORD: ${TECHNITIUM_ADMIN_PASSWORD}
    ports:
      - "${TECHNITIUM_DNS_PORT}:53/udp"
      - "${TECHNITIUM_DNS_PORT}:53/tcp"
      - "127.0.0.1:${TECHNITIUM_HTTP_PORT}:5380"
    volumes:
      - ${TECHNITIUM_DIR}/config:/etc/dns
    networks:
      - ${TRAEFIK_PUBLIC_NETWORK}
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=${TRAEFIK_PUBLIC_NETWORK}"
      - "traefik.http.routers.technitium.rule=Host(\`${ORBYTALS_DNS_DOMAIN}\`)"
      - "traefik.http.routers.technitium.entrypoints=websecure"
      - "traefik.http.routers.technitium.tls.certresolver=le"
      - "traefik.http.routers.technitium.middlewares=secure-headers@file"
      - "traefik.http.services.technitium.loadbalancer.server.port=5380"

networks:
  ${TRAEFIK_PUBLIC_NETWORK}:
    external: true
EOF
}

install_technitium() {
  log "Installing Technitium"
  write_technitium_compose
  compose_up_or_die "${TECHNITIUM_DIR}" "Technitium" "technitium"
}

write_forgejo_compose() {
  ensure_dir "${FORGEJO_DIR}/data"
  chown -R 1000:1000 "${FORGEJO_DIR}/data" || true
  cat >"${FORGEJO_COMPOSE_FILE}" <<EOF
services:
  forgejo:
    image: codeberg.org/forgejo/forgejo:9
    container_name: forgejo
    restart: unless-stopped
    environment:
      USER_UID: 1000
      USER_GID: 1000
      FORGEJO__server__ROOT_URL: https://${ORBYTALS_GIT_DOMAIN}/
      FORGEJO__server__DOMAIN: ${ORBYTALS_GIT_DOMAIN}
      FORGEJO__server__SSH_DOMAIN: ${ORBYTALS_GIT_DOMAIN}
      FORGEJO__server__HTTP_PORT: 3000
      FORGEJO__server__SSH_PORT: ${FORGEJO_SSH_PORT}
      FORGEJO__server__START_SSH_SERVER: "true"
      FORGEJO__service__DISABLE_REGISTRATION: "false"
    ports:
      - "${FORGEJO_SSH_PORT}:22"
    volumes:
      - ${FORGEJO_DIR}/data:/data
    networks:
      - ${TRAEFIK_PUBLIC_NETWORK}
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=${TRAEFIK_PUBLIC_NETWORK}"
      - "traefik.http.routers.forgejo.rule=Host(\`${ORBYTALS_GIT_DOMAIN}\`)"
      - "traefik.http.routers.forgejo.entrypoints=websecure"
      - "traefik.http.routers.forgejo.tls.certresolver=le"
      - "traefik.http.routers.forgejo.middlewares=secure-headers@file"
      - "traefik.http.services.forgejo.loadbalancer.server.port=3000"

networks:
  ${TRAEFIK_PUBLIC_NETWORK}:
    external: true
EOF
}

install_forgejo() {
  log "Installing Forgejo"
  write_forgejo_compose
  compose_up_or_die "${FORGEJO_DIR}" "Forgejo" "forgejo"
  if wait_for_container_running forgejo 60; then
    docker exec forgejo /usr/local/bin/forgejo admin user create \
    --admin \
    --username "${ADMIN_USERNAME}" \
    --password "${ADMIN_PASSWORD}" \
    --email "${FORGEJO_ADMIN_EMAIL}" >/dev/null 2>&1 || true
  else
    docker logs forgejo --tail 200 >&2 || true
    die "Forgejo container not running"
  fi
}

patch_synapse_yaml() {
  local sy="$1"
  cp -a "$sy" "${sy}.bak.$(date +%s)" || true
  if grep -qE '^public_baseurl:' "$sy"; then
    sed -i 's|^public_baseurl:.*|public_baseurl: "https://'"${HERMES_CHAT_DOMAIN}"'/"|' "$sy"
  else
    printf '\npublic_baseurl: "https://%s/"\n' "${HERMES_CHAT_DOMAIN}" >>"$sy"
  fi
  if grep -qE '^serve_server_wellknown:' "$sy"; then
    sed -i 's|^serve_server_wellknown:.*|serve_server_wellknown: true|' "$sy"
  else
    printf 'serve_server_wellknown: true\n' >>"$sy"
  fi
  if grep -qE '^registration_shared_secret:' "$sy"; then
    sed -i 's|^registration_shared_secret:.*|registration_shared_secret: "'"${MATRIX_REGISTRATION_SHARED_SECRET}"'"|' "$sy"
  else
    printf 'registration_shared_secret: "%s"\n' "${MATRIX_REGISTRATION_SHARED_SECRET}" >>"$sy"
  fi
}

generate_synapse_config() {
  ensure_dir "${SYNAPSE_CONFIG_DIR}"
  if [[ ! -f "${SYNAPSE_CONFIG_DIR}/homeserver.yaml" ]]; then
    docker run --rm \
      -e SYNAPSE_SERVER_NAME="${HERMES_CHAT_DOMAIN}" \
      -e SYNAPSE_REPORT_STATS=no \
      -v "${SYNAPSE_CONFIG_DIR}:/data" \
      matrixdotorg/synapse:latest generate
  fi
  patch_synapse_yaml "${SYNAPSE_CONFIG_DIR}/homeserver.yaml"
}

write_synapse_compose() {
  ensure_dir "${SYNAPSE_DIR}"
  cat >"${SYNAPSE_COMPOSE_FILE}" <<EOF
services:
  ${SYNAPSE_SERVICE_NAME}:
    image: matrixdotorg/synapse:latest
    container_name: ${SYNAPSE_SERVICE_NAME}
    restart: unless-stopped
    environment:
      SYNAPSE_CONFIG_PATH: /data/homeserver.yaml
    volumes:
      - ${SYNAPSE_CONFIG_DIR}:/data
    networks:
      - ${TRAEFIK_PUBLIC_NETWORK}
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=${TRAEFIK_PUBLIC_NETWORK}"
      - "traefik.http.routers.synapse.rule=Host(\`${HERMES_CHAT_DOMAIN}\`)"
      - "traefik.http.routers.synapse.entrypoints=websecure"
      - "traefik.http.routers.synapse.tls.certresolver=le"
      - "traefik.http.routers.synapse.middlewares=secure-headers@file"
      - "traefik.http.services.synapse.loadbalancer.server.port=${SYNAPSE_HTTP_PORT}"

networks:
  ${TRAEFIK_PUBLIC_NETWORK}:
    external: true
EOF
}

install_synapse() {
  log "Installing Synapse"
  generate_synapse_config
  write_synapse_compose
  compose_up_or_die "${SYNAPSE_DIR}" "Synapse" "${SYNAPSE_SERVICE_NAME}"
  wait_for_container_running "${SYNAPSE_SERVICE_NAME}" 60 || {
    docker logs "${SYNAPSE_SERVICE_NAME}" --tail 200 >&2 || true
    die "Synapse container not running"
  }
  docker run --rm \
    --network "${TRAEFIK_PUBLIC_NETWORK}" \
    -v "${SYNAPSE_CONFIG_DIR}:/data" \
    matrixdotorg/synapse:latest \
    register_new_matrix_user \
      -u "${ADMIN_USERNAME}" \
      -p "${ADMIN_PASSWORD}" \
      -a \
      -k "${MATRIX_REGISTRATION_SHARED_SECRET}" \
      "http://${SYNAPSE_SERVICE_NAME}:${SYNAPSE_HTTP_PORT}" >/dev/null 2>&1 || true
}

ensure_mailcow_clone() {
  if [[ ! -d "${MAILCOW_DIR}/.git" ]]; then
    git clone --depth 1 --branch "${MAILCOW_BRANCH}" "${MAILCOW_CLONE_URL}" "${MAILCOW_DIR}"
  else
    git -C "${MAILCOW_DIR}" fetch --depth 1 origin "${MAILCOW_BRANCH}"
    git -C "${MAILCOW_DIR}" checkout "${MAILCOW_BRANCH}"
    git -C "${MAILCOW_DIR}" pull --ff-only origin "${MAILCOW_BRANCH}"
  fi
}

ensure_mailcow_config() {
  local conf="${MAILCOW_DIR}/mailcow.conf"
  if [[ ! -f "$conf" ]]; then
    (
      cd "${MAILCOW_DIR}"
      printf '%s\n' "${ORBYTALS_MAIL_DOMAIN}" | ./generate_config.sh
    )
  fi
  ensure_conf_kv "$conf" MAILCOW_HOSTNAME "${ORBYTALS_MAIL_DOMAIN}"
  ensure_conf_kv "$conf" SKIP_LETS_ENCRYPT "y"
  ensure_conf_kv "$conf" HTTP_BIND "${MAILCOW_HTTP_BIND}"
  ensure_conf_kv "$conf" HTTP_PORT "${MAILCOW_HTTP_PORT}"
  ensure_conf_kv "$conf" HTTPS_BIND "${MAILCOW_HTTPS_BIND}"
  ensure_conf_kv "$conf" HTTPS_PORT "${MAILCOW_HTTPS_PORT}"
  ensure_conf_kv "$conf" DOCKER_COMPOSE_VERSION "native"
  ensure_conf_kv "$conf" TZ "${MAILCOW_TIMEZONE}"
}

write_mailcow_override() {
  cat >"${MAILCOW_DIR}/${MAILCOW_TRAEFIK_OVERRIDE_FILE}" <<EOF
services:
  nginx-mailcow:
    networks:
      traefik_edge: {}
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=${TRAEFIK_PUBLIC_NETWORK}"
      - "traefik.http.routers.mailcow.rule=Host(\`${ORBYTALS_MAIL_DOMAIN}\`)"
      - "traefik.http.routers.mailcow.entrypoints=websecure"
      - "traefik.http.routers.mailcow.tls.certresolver=le"
      - "traefik.http.routers.mailcow.middlewares=secure-headers@file"
      - "traefik.http.services.mailcow.loadbalancer.server.port=${MAILCOW_HTTP_PORT}"

networks:
  traefik_edge:
    name: ${TRAEFIK_PUBLIC_NETWORK}
    external: true
EOF
}

mailcow_compose_files() {
  local base="${MAILCOW_DIR}/docker-compose.yml"
  local primary_override="${MAILCOW_DIR}/docker-compose.override.yml"
  local managed_override="${MAILCOW_DIR}/${MAILCOW_TRAEFIK_OVERRIDE_FILE}"
  local args=(-f "$base")
  if [[ -f "$primary_override" ]]; then
    args+=(-f "$primary_override")
  fi
  args+=(-f "$managed_override")
  printf '%s\n' "${args[@]}"
}

mailcow_compose() {
  local args=()
  while IFS= read -r line; do
    args+=("$line")
  done < <(mailcow_compose_files)
  (cd "${MAILCOW_DIR}" && docker compose "${args[@]}" "$@")
}

install_mailcow() {
  log "Installing Mailcow"
  ensure_mailcow_clone
  ensure_mailcow_config
  write_mailcow_override
  mailcow_compose pull || {
    mailcow_compose ps >&2 || true
    die "Mailcow pull failed"
  }
  mailcow_compose up -d || {
    mailcow_compose ps >&2 || true
    docker logs nginx-mailcow --tail 200 >&2 || true
    die "Mailcow failed to start"
  }
}

show_port_status() {
  log "Port status"
  ss -ltnup | awk 'NR==1 || /:22 |:25 |:8053 |:80 |:110 |:143 |:443 |:465 |:587 |:993 |:995 |:2222 |:8080 |:8082 |:8085 |:8444 |:8448 |:9292 |:18080 |:3900 |:3902 |:3903 /'
}

verify_url() {
  local url="$1"
  echo "---- $url"
  if curl -fsSI --max-time 20 "$url" >/dev/null 2>&1; then
    curl -fsSI --max-time 20 "$url"
    return
  fi
  if curl -kfsSI --max-time 20 "$url" >/dev/null 2>&1; then
    curl -kfsSI --max-time 20 "$url"
    echo "(reachable with insecure TLS; ACME may still be pending)"
    return
  fi
  echo "(failed)"
}

# Hit Traefik on loopback with correct SNI so checks work without hairpin-NAT and before public DNS is ready.
verify_traefik_https() {
  local host="$1"
  local path="${2:-/}"
  [[ "${path}" == /* ]] || path="/${path}"
  local url="https://${host}${path}"
  echo "---- ${url} (Traefik @ 127.0.0.1:443, SNI ${host})"
  local out
  out="$(curl -sSI --max-time 25 --resolve "${host}:443:127.0.0.1" "$url" 2>&1)" || true
  if printf '%s' "$out" | grep -qE '^HTTP/[0-9.]+ '; then
    printf '%s\n' "$out" | head -n 25
    return 0
  fi
  out="$(curl -ksSI --max-time 25 --resolve "${host}:443:127.0.0.1" "$url" 2>&1)" || true
  if printf '%s' "$out" | grep -qE '^HTTP/[0-9.]+ '; then
    printf '%s\n' "$out" | head -n 25
    echo "(TLS certificate verify skipped; fix ACME or trust chain if this is unexpected)"
    return 0
  fi
  echo "(failed — Traefik not on 443, no router for Host, or connection error)"
  printf '%s\n' "$out" | tail -n 10
}

# Garage S3 API often returns 4xx on HEAD /; curl -f would false-fail. Accept any HTTP response or open TCP port.
verify_garage_s3_endpoint() {
  local base="${GARAGE_ENDPOINT_URL}"
  echo "---- ${base} (Garage S3 API)"
  local out
  out="$(curl -sSI --max-time 15 "${base}/" 2>&1)" || true
  if printf '%s' "$out" | grep -qE '^HTTP/[0-9.]+ '; then
    printf '%s\n' "$out" | head -n 18
    return 0
  fi
  if bash -c "exec 3<>/dev/tcp/${DEPLOYWERK_LOOPBACK_HOST}/${GARAGE_S3_PORT}" 2>/dev/null; then
    exec 3<&- 3>&- 2>/dev/null || true
    echo "TCP ${DEPLOYWERK_LOOPBACK_HOST}:${GARAGE_S3_PORT} open (Garage listening)"
    return 0
  fi
  echo "(failed)"
  printf '%s\n' "$out" | tail -n 8
}

verify_install() {
  show_port_status
  verify_traefik_https "${ORBYTALS_APP_DOMAIN}" "/"
  verify_traefik_https "${ORBYTALS_API_DOMAIN}" "/api/v1/bootstrap"
  verify_traefik_https "${ORBYTALS_MAIL_DOMAIN}" "/"
  verify_traefik_https "${ORBYTALS_GIT_DOMAIN}" "/"
  verify_traefik_https "${ORBYTALS_DNS_DOMAIN}" "/"
  verify_traefik_https "${ORBYTALS_TRAEFIK_DOMAIN}" "/"
  verify_traefik_https "${ORBYTALS_COCKPIT_DOMAIN}" "/"
  verify_traefik_https "${HERMES_CHAT_DOMAIN}" "/_matrix/client/versions"
  verify_traefik_https "${HERMES_CHAT_DOMAIN}" "/.well-known/matrix/server"
  verify_url "http://${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_NGINX_PORT}"
  verify_url "http://${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_API_PORT}/api/v1/health"
  verify_garage_s3_endpoint
}

compose_down_if_present() {
  local dir="$1"
  if [[ -f "${dir}/docker-compose.yml" ]]; then
    (cd "$dir" && docker compose down --remove-orphans) || true
  fi
}

clean_install() {
  log "Cleaning managed services"
  compose_down_if_present "${EDGE_ROOT}/traefik"
  compose_down_if_present "${GARAGE_DIR}"
  compose_down_if_present "${TECHNITIUM_DIR}"
  compose_down_if_present "${FORGEJO_DIR}"
  compose_down_if_present "${SYNAPSE_DIR}"
  if [[ -d "${MAILCOW_DIR}" ]]; then
    mailcow_compose down --remove-orphans || true
  fi

  systemctl disable --now deploywerk-api >/dev/null 2>&1 || true
  systemctl disable --now deploywerk-deploy-worker >/dev/null 2>&1 || true
  rm -f "${DEPLOYWERK_API_SERVICE_FILE}" "${DEPLOYWERK_WORKER_SERVICE_FILE}"
  rm -f "${DEPLOYWERK_NGINX_ENABLED_SITE}" "${DEPLOYWERK_NGINX_SITE}"
  systemctl daemon-reload || true
  systemctl restart nginx >/dev/null 2>&1 || true

  rm -rf "${INSTALL_ROOT}" "${MAILCOW_DIR}" "${STATE_DIR}" "${DEPLOYWERK_WEB_ROOT}"
  docker network rm "${TRAEFIK_PUBLIC_NETWORK}" >/dev/null 2>&1 || true
  echo "Managed Orbytals install cleaned."
}

cmd_install() {
  require_root
  collect_inputs
  preflight_ports
  bootstrap_host
  install_traefik
  install_garage
  bootstrap_garage
  install_technitium
  install_forgejo
  install_synapse
  install_mailcow
  install_native_deploywerk
}

cmd_all() {
  cmd_install
  verify_install
}

usage() {
  cat <<EOF
Commands:
  install   Install or update the full Orbytals stack
  verify    Verify managed services and public URLs
  clean     Remove the managed Orbytals install footprint
  all       Install/update then verify

Primary command:
  sudo bash scripts/orbytals-install.sh all
EOF
}

main() {
  local sub="${1:-}"
  case "$sub" in
    install) shift; cmd_install "$@" ;;
    verify) shift; verify_install "$@" ;;
    clean) shift; clean_install "$@" ;;
    all) shift; cmd_all "$@" ;;
    ""|-h|--help) usage ;;
    *) die "unknown command: $sub" ;;
  esac
}

main "$@"
