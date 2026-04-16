#!/usr/bin/env bash
# Bootstrap the Orbytals edge stack and native DeployWerk install on Ubuntu.
#
# This script keeps Traefik and Mailcow in Docker, while installing DeployWerk,
# Cockpit, nginx, PostgreSQL, and systemd units on the host.
#
# Typical usage:
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh install
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh up
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh native-deploywerk-install
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh mailcow-install
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh minio-bootstrap
#   sudo ./scripts/traefik-edge-migrate-orbytals.sh apply-labels
#   ./scripts/traefik-edge-migrate-orbytals.sh verify

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
SOURCE_TREE="${SOURCE_TREE:-${REPO_ROOT}/examples/orbytals-traefik-edge}"
EDGE_ROOT="${EDGE_ROOT:-/opt/orbytals/edge}"

TRAEFIK_PUBLIC_NETWORK="${TRAEFIK_PUBLIC_NETWORK:-proxy}"
MAILCOW_TRAEFIK_NETWORK="${MAILCOW_TRAEFIK_NETWORK:-proxy}"
MAILCOW_NGINX_SERVICE="${MAILCOW_NGINX_SERVICE:-nginx-mailcow}"
TECHNITIUM_SERVICE="${TECHNITIUM_SERVICE:-technitium}"

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
MAILCOW_HOSTNAME="${MAILCOW_HOSTNAME:-${ORBYTALS_MAIL_DOMAIN}}"
MAILCOW_TIMEZONE="${MAILCOW_TIMEZONE:-UTC}"
MAILCOW_TRAEFIK_OVERRIDE_FILE="${MAILCOW_TRAEFIK_OVERRIDE_FILE:-docker-compose.orbytals-traefik.yml}"
MAILCOW_HTTP_BIND="${MAILCOW_HTTP_BIND:-127.0.0.1}"
MAILCOW_HTTP_PORT="${MAILCOW_HTTP_PORT:-8082}"
MAILCOW_HTTPS_BIND="${MAILCOW_HTTPS_BIND:-127.0.0.1}"
MAILCOW_HTTPS_PORT="${MAILCOW_HTTPS_PORT:-8444}"

MINIO_ALIAS_NAME="${MINIO_ALIAS_NAME:-local}"
MINIO_ENDPOINT_URL="${MINIO_ENDPOINT_URL:-http://127.0.0.1:9000}"
MINIO_ROOT_USER="${MINIO_ROOT_USER:-deploywerk}"
MINIO_ROOT_PASSWORD="${MINIO_ROOT_PASSWORD:-deploywerk-dev-only-change-me}"
MINIO_BUCKET_NAME="${MINIO_BUCKET_NAME:-deploywerk}"

die() {
  echo "error: $*" >&2
  exit 1
}

require_root_for() {
  if [[ "${EUID}" -ne 0 ]]; then
    die "this command must be run as root: $*"
  fi
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

ensure_env_kv() {
  local file="$1" key="$2" val="$3"
  mkdir -p "$(dirname "$file")"
  touch "$file"
  local tmp
  tmp="$(mktemp)"
  grep -v "^${key}=" "$file" >"$tmp" 2>/dev/null || true
  printf '%s=%s\n' "$key" "$val" >>"$tmp"
  mv "$tmp" "$file"
}

env_value() {
  local file="$1" key="$2"
  awk -F= -v key="$key" '$1 == key {sub(/^[^=]*=/, "", $0); print $0}' "$file" | tail -n 1
}

ensure_env_secret() {
  local file="$1" key="$2" generator="$3"
  local current
  current="$(env_value "$file" "$key" || true)"
  if [[ -z "${current}" || "${current}" == "ReplaceWithMailcowGeneratedMailboxPassword" || "${current}" == "ptr_ReplaceWithPortainerAccessToken" || "${current}" == "ReplaceWithTechnitiumDnsApiToken" || "${current}" == "authentik-secret-key-minimum-fifty-characters-long-change-me-in-production-please" || "${current}" == "authentik-postgres-password-change-me" || "${current}" == "deploywerk-dev-only-change-me" || "${current}" == "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" || "${current}" == "nL4qK8vR2mP9sT5wX1yZ6aB3cD7eF0gH4jM8nQ2rS5tU9vW1xY4zA6bC8dE2fG5h" ]]; then
    ensure_env_kv "$file" "$key" "$($generator)"
  fi
}

ensure_conf_kv() {
  local file="$1" key="$2" val="$3"
  mkdir -p "$(dirname "$file")"
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

copy_tree() {
  [[ -d "$SOURCE_TREE" ]] || die "SOURCE_TREE is not a directory: $SOURCE_TREE"
  mkdir -p "$EDGE_ROOT"
  if command_exists rsync; then
    rsync -a "${SOURCE_TREE}/" "${EDGE_ROOT}/"
  else
    cp -a "${SOURCE_TREE}/." "${EDGE_ROOT}/"
  fi
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

ensure_traefik_env_kv() {
  ensure_env_kv "${EDGE_ROOT}/traefik/.env" "$1" "$2"
}

gen_dashboard_auth() {
  local out="${EDGE_ROOT}/traefik/dynamic/dashboard-auth.yml"
  local dash="${EDGE_ROOT}/traefik/dynamic/dashboard.yml"
  command_exists htpasswd || die "htpasswd not found (install apache2-utils)"
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
  echo "Wrote ${out} and updated ${dash} (dashboard basic auth enabled)."
}

ensure_deploywerk_user() {
  if ! id -u "${DEPLOYWERK_USER}" >/dev/null 2>&1; then
    useradd --system --create-home --home-dir "${DEPLOYWERK_HOME}" --shell /usr/sbin/nologin "${DEPLOYWERK_USER}"
  fi
  mkdir -p "${DEPLOYWERK_STATE_ROOT}/git-cache" "${DEPLOYWERK_STATE_ROOT}/volumes" "${DEPLOYWERK_WEB_ROOT}" /etc/deploywerk
  chown -R "${DEPLOYWERK_USER}:${DEPLOYWERK_USER}" "${DEPLOYWERK_STATE_ROOT}"
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

ensure_postgres_db() {
  if ! systemctl is-active --quiet postgresql; then
    systemctl enable --now postgresql
  fi
  su - postgres -s /bin/sh -c "psql -tAc \"SELECT 1 FROM pg_roles WHERE rolname='${DEPLOYWERK_DB_USER}'\" | grep -q 1" \
    || su - postgres -s /bin/sh -c "psql -c \"CREATE USER \\\"${DEPLOYWERK_DB_USER}\\\" WITH PASSWORD '${DEPLOYWERK_DB_PASSWORD}';\""
  su - postgres -s /bin/sh -c "psql -tAc \"SELECT 1 FROM pg_database WHERE datname='${DEPLOYWERK_DB_NAME}'\" | grep -q 1" \
    || su - postgres -s /bin/sh -c "createdb -O \"${DEPLOYWERK_DB_USER}\" \"${DEPLOYWERK_DB_NAME}\""
}

ensure_deploywerk_env() {
  if [[ ! -f "${DEPLOYWERK_ENV_FILE}" ]]; then
    cp "${REPO_ROOT}/.env.example" "${DEPLOYWERK_ENV_FILE}"
  fi

  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" APP_ENV production
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" HOST "${DEPLOYWERK_LOOPBACK_HOST}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" PORT "${DEPLOYWERK_API_PORT}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DATABASE_URL "postgresql://${DEPLOYWERK_DB_USER}:${DEPLOYWERK_DB_PASSWORD}@127.0.0.1:5432/${DEPLOYWERK_DB_NAME}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_PUBLIC_APP_URL "https://${ORBYTALS_APP_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_API_URL "https://${ORBYTALS_API_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" VITE_API_URL "https://${ORBYTALS_API_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_API_PROXY "http://${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_API_PORT}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_GIT_SHA "orbytals-native"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_GIT_CACHE_ROOT "${DEPLOYWERK_STATE_ROOT}/git-cache"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_VOLUMES_ROOT "${DEPLOYWERK_STATE_ROOT}/volumes"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_ENDPOINT_URL "${MINIO_ENDPOINT_URL}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_BUCKET "${MINIO_BUCKET_NAME}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_REGION "us-east-1"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_PATH_STYLE "true"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_ACCESS_KEY "${MINIO_ROOT_USER}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEFAULT_STORAGE_SECRET_KEY "${MINIO_ROOT_PASSWORD}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_PLATFORM_DOCKER_ENABLED "true"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_APPS_BASE_DOMAIN "${ORBYTALS_APEX_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_EDGE_MODE "traefik"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_TRAEFIK_DOCKER_NETWORK "${TRAEFIK_PUBLIC_NETWORK}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_LOCAL_SERVICE_DEFAULTS "true"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_TRAEFIK_URL "http://127.0.0.1:8080"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_FORGEJO_URL "https://${ORBYTALS_GIT_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_TECHNITIUM_URL "https://${ORBYTALS_DNS_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_MAILCOW_URL "https://${ORBYTALS_MAIL_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_INTEGRATION_MATRIX_CLIENT_URL "https://${HERMES_CHAT_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_HOST "127.0.0.1"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_PORT "587"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_USER "deploywerk@${ORBYTALS_APEX_DOMAIN}"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_FROM "DeployWerk <deploywerk@${ORBYTALS_APEX_DOMAIN}>"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_SMTP_TLS "starttls"
  ensure_env_kv "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_MAIL_ENABLED "false"

  ensure_env_secret "${DEPLOYWERK_ENV_FILE}" JWT_SECRET generate_jwt_secret
  ensure_env_secret "${DEPLOYWERK_ENV_FILE}" SERVER_KEY_ENCRYPTION_KEY generate_hex32
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
  server_name ${ORBYTALS_APEX_DOMAIN} ${ORBYTALS_APP_DOMAIN} ${ORBYTALS_API_DOMAIN};
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
  command_exists npm || die "npm not found; run scripts/server-bootstrap-orbytals.sh first"
  (cd "${REPO_ROOT}" && cargo build --release -p deploywerk-api --bin deploywerk-api --bin deploywerk-deploy-worker)
  install -m 0755 "${REPO_ROOT}/target/release/deploywerk-api" "${DEPLOYWERK_API_BIN}"
  install -m 0755 "${REPO_ROOT}/target/release/deploywerk-deploy-worker" "${DEPLOYWERK_WORKER_BIN}"
  (cd "${REPO_ROOT}/web" && npm ci && npm run build)
  mkdir -p "${DEPLOYWERK_WEB_ROOT}"
  cp -a "${REPO_ROOT}/web/dist/." "${DEPLOYWERK_WEB_ROOT}/"
  chown -R "${DEPLOYWERK_USER}:${DEPLOYWERK_USER}" "${DEPLOYWERK_STATE_ROOT}" "${DEPLOYWERK_WEB_ROOT}"
}

worker_dispatch_external() {
  [[ "$(env_value "${DEPLOYWERK_ENV_FILE}" DEPLOYWERK_DEPLOY_DISPATCH || true)" == "external" ]]
}

write_mailcow_override() {
  local dest="$1"
  cat >"$dest" <<EOF
services:
  ${MAILCOW_NGINX_SERVICE}:
    networks:
      traefik_edge: {}
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=${MAILCOW_TRAEFIK_NETWORK}"
      - "traefik.http.routers.mailcow.rule=Host(\`${ORBYTALS_MAIL_DOMAIN}\`)"
      - "traefik.http.routers.mailcow.entrypoints=websecure"
      - "traefik.http.routers.mailcow.tls.certresolver=le"
      - "traefik.http.routers.mailcow.service=mailcow-svc"
      - "traefik.http.routers.mailcow.middlewares=secure-headers@file"
      - "traefik.http.services.mailcow-svc.loadbalancer.server.port=${MAILCOW_HTTP_PORT}"

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
services:
  ${TECHNITIUM_SERVICE}:
    networks:
      traefik_edge: {}
    labels:
      - "traefik.enable=true"
      - "traefik.docker.network=${TRAEFIK_PUBLIC_NETWORK}"
      - "traefik.http.routers.technitium.rule=Host(\`${ORBYTALS_DNS_DOMAIN}\`)"
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
    [[ -x "${MAILCOW_DIR}/generate_config.sh" ]] || die "mailcow generate_config.sh not found"
    (
      cd "${MAILCOW_DIR}"
      printf '%s\n' "${MAILCOW_HOSTNAME}" | ./generate_config.sh
    )
  fi
  ensure_conf_kv "$conf" MAILCOW_HOSTNAME "${MAILCOW_HOSTNAME}"
  ensure_conf_kv "$conf" SKIP_LETS_ENCRYPT "y"
  ensure_conf_kv "$conf" HTTP_BIND "${MAILCOW_HTTP_BIND}"
  ensure_conf_kv "$conf" HTTP_PORT "${MAILCOW_HTTP_PORT}"
  ensure_conf_kv "$conf" HTTPS_BIND "${MAILCOW_HTTPS_BIND}"
  ensure_conf_kv "$conf" HTTPS_PORT "${MAILCOW_HTTPS_PORT}"
  ensure_conf_kv "$conf" DOCKER_COMPOSE_VERSION "native"
  ensure_conf_kv "$conf" TZ "${MAILCOW_TIMEZONE}"
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
  echo "Patched Synapse $(basename "$sy") for public_baseurl and serve_server_wellknown."
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

cmd_native_deploywerk_install() {
  require_root_for native-deploywerk-install
  ensure_deploywerk_user
  ensure_postgres_db
  ensure_deploywerk_env
  build_native_deploywerk
  write_deploywerk_api_service
  write_deploywerk_worker_service
  write_nginx_site
  nginx -t
  systemctl daemon-reload
  systemctl enable --now nginx
  systemctl enable --now deploywerk-api
  if worker_dispatch_external; then
    systemctl enable --now deploywerk-deploy-worker
  else
    systemctl disable --now deploywerk-deploy-worker >/dev/null 2>&1 || true
  fi
  systemctl reload nginx
  echo "DeployWerk installed natively."
  echo "  API unit: deploywerk-api"
  echo "  Env file: ${DEPLOYWERK_ENV_FILE}"
  echo "  Nginx loopback: http://${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_NGINX_PORT}"
}

cmd_mailcow_install() {
  require_root_for mailcow-install
  ensure_mailcow_clone
  ensure_mailcow_config
  write_mailcow_override "${MAILCOW_DIR}/${MAILCOW_TRAEFIK_OVERRIDE_FILE}"
  docker network create "${TRAEFIK_PUBLIC_NETWORK}" >/dev/null 2>&1 || true
  mailcow_compose pull
  mailcow_compose up -d
  echo "Mailcow installed or updated in ${MAILCOW_DIR}."
  echo "  HTTPS is bound to ${MAILCOW_HTTPS_BIND}:${MAILCOW_HTTPS_PORT} for host-local access."
  echo "  Traefik labels are in ${MAILCOW_DIR}/${MAILCOW_TRAEFIK_OVERRIDE_FILE}."
}

cmd_minio_bootstrap() {
  require_root_for minio-bootstrap
  local bucket_uri="${MINIO_ALIAS_NAME}/${MINIO_BUCKET_NAME}"
  local shell_cmd
  shell_cmd=$(
    cat <<EOF
set -eu
mc alias set ${MINIO_ALIAS_NAME} ${MINIO_ENDPOINT_URL} ${MINIO_ROOT_USER} ${MINIO_ROOT_PASSWORD} >/dev/null
if mc ls "${bucket_uri}" >/dev/null 2>&1; then
  echo "Bucket already exists: ${bucket_uri}"
else
  mc mb --ignore-existing "${bucket_uri}" >/dev/null
  echo "Bucket created successfully \`${bucket_uri}\`."
fi
EOF
  )
  docker run --rm --network host minio/mc:latest /bin/sh -lc "$shell_cmd"
}

cmd_dns() {
  cat <<EOF
DNS checklist (all should point at the Traefik host IP):

  TYPE   NAME                     VALUE
  A      ${ORBYTALS_APEX_DOMAIN}   <SERVER_IP>
  A      *.orbytals.com            <SERVER_IP>
  A      hermesapp.live            <SERVER_IP>
  A      *.hermesapp.live          <SERVER_IP>

Recommended hostnames used by this automation:
  ${ORBYTALS_APP_DOMAIN}
  ${ORBYTALS_API_DOMAIN}
  ${ORBYTALS_MAIL_DOMAIN}
  ${ORBYTALS_GIT_DOMAIN}
  ${ORBYTALS_DNS_DOMAIN}
  ${ORBYTALS_TRAEFIK_DOMAIN}
  ${ORBYTALS_COCKPIT_DOMAIN}

Mail records:
  MX     ${ORBYTALS_APEX_DOMAIN}   10 ${ORBYTALS_MAIL_DOMAIN}
  TXT    ${ORBYTALS_APEX_DOMAIN}   "v=spf1 mx ~all"

DKIM + DMARC: configure in Mailcow after first boot.
EOF
}

cmd_verify() {
  set +e
  local urls=(
    "https://${ORBYTALS_APP_DOMAIN}"
    "https://${ORBYTALS_API_DOMAIN}/api/v1/health"
    "https://${ORBYTALS_MAIL_DOMAIN}"
    "https://${ORBYTALS_GIT_DOMAIN}"
    "https://${ORBYTALS_DNS_DOMAIN}"
    "https://${ORBYTALS_COCKPIT_DOMAIN}"
    "https://${HERMES_CHAT_DOMAIN}/_matrix/client/versions"
    "https://${HERMES_CHAT_DOMAIN}/.well-known/matrix/server"
    "http://${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_API_PORT}/api/v1/health"
    "http://${DEPLOYWERK_LOOPBACK_HOST}:${DEPLOYWERK_NGINX_PORT}"
    "${MINIO_ENDPOINT_URL}/minio/health/live"
  )
  local u
  for u in "${urls[@]}"; do
    echo "---- $u"
    curl -fsSI --max-time 15 "$u" || echo "(failed — check DNS / TLS / container health / native service status)"
  done
  set -e
}

cmd_apply_labels() {
  require_root_for apply-labels
  if [[ -n "${MAILCOW_DIR:-}" && -d "${MAILCOW_DIR}" ]]; then
    write_mailcow_override "${MAILCOW_DIR}/${MAILCOW_TRAEFIK_OVERRIDE_FILE}"
    if [[ -f "${MAILCOW_DIR}/mailcow.conf" ]]; then
      ensure_conf_kv "${MAILCOW_DIR}/mailcow.conf" SKIP_LETS_ENCRYPT "y"
      ensure_conf_kv "${MAILCOW_DIR}/mailcow.conf" HTTP_BIND "${MAILCOW_HTTP_BIND}"
      ensure_conf_kv "${MAILCOW_DIR}/mailcow.conf" HTTP_PORT "${MAILCOW_HTTP_PORT}"
      ensure_conf_kv "${MAILCOW_DIR}/mailcow.conf" HTTPS_BIND "${MAILCOW_HTTPS_BIND}"
      ensure_conf_kv "${MAILCOW_DIR}/mailcow.conf" HTTPS_PORT "${MAILCOW_HTTPS_PORT}"
    fi
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
      sed -i.bak "s|^ROOT_URL=.*|ROOT_URL=https://${ORBYTALS_GIT_DOMAIN}/|" "${FORGEJO_APP_INI}" && rm -f "${FORGEJO_APP_INI}.bak"
    else
      printf '\nROOT_URL=https://%s/\n' "${ORBYTALS_GIT_DOMAIN}" >>"${FORGEJO_APP_INI}"
    fi
    echo "Updated ROOT_URL in ${FORGEJO_APP_INI}."
  elif [[ -n "${FORGEJO_DATA_DIR:-}" ]]; then
    local fd="${FORGEJO_DATA_DIR%/}"
    local ini="${fd}/custom/conf/app.ini"
    [[ -f "$ini" ]] || die "expected Forgejo app.ini at $ini"
    if grep -q '^ROOT_URL=' "$ini"; then
      sed -i.bak "s|^ROOT_URL=.*|ROOT_URL=https://${ORBYTALS_GIT_DOMAIN}/|" "$ini" && rm -f "${ini}.bak"
    else
      printf '\nROOT_URL=https://%s/\n' "${ORBYTALS_GIT_DOMAIN}" >>"$ini"
    fi
    echo "Updated ROOT_URL in $ini."
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

cmd_all() {
  cmd_install
  cmd_stop_legacy
  cmd_up
  cmd_native_deploywerk_install
  cmd_mailcow_install
  cmd_minio_bootstrap
  cmd_apply_labels
  cmd_dns
  echo "Running verify (may fail until DNS, ACME, and upstream services are live)..."
  cmd_verify
}

usage() {
  cat <<EOF
Commands:
  install
  stop-legacy
  up [--follow-logs]
  native-deploywerk-install
  mailcow-install
  minio-bootstrap
  apply-labels
  verify
  dns
  all

Environment:
  SOURCE_TREE               default: <repo>/examples/orbytals-traefik-edge
  EDGE_ROOT                 default: /opt/orbytals/edge
  TRAEFIK_PUBLIC_NETWORK    default: proxy
  MAILCOW_DIR               default: /opt/mailcow-dockerized
  MAILCOW_HOSTNAME          default: ${ORBYTALS_MAIL_DOMAIN}
  DEPLOYWERK_ENV_FILE       default: /etc/deploywerk/deploywerk.env
  DEPLOYWERK_DB_PASSWORD    default: deploywerk
  MINIO_ENDPOINT_URL        default: http://127.0.0.1:9000

Optional apply-labels:
  TECHNITIUM_COMPOSE_DIR, FORGEJO_APP_INI or FORGEJO_DATA_DIR,
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
    native-deploywerk-install) cmd_native_deploywerk_install ;;
    mailcow-install) cmd_mailcow_install ;;
    minio-bootstrap) cmd_minio_bootstrap ;;
    apply-labels) cmd_apply_labels ;;
    verify) cmd_verify ;;
    dns) cmd_dns ;;
    all) cmd_all ;;
    "" | -h | --help) usage ;;
    *) die "unknown command: $sub" ;;
  esac
}

main "$@"
