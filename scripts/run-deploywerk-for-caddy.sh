#!/usr/bin/env bash
# Run DeployWerk API + loopback nginx on configurable ports for Caddy (or similar).
#
# Example Caddy site (global ACME/email is separate):
#   deploywerk.orbytals.com {
#       reverse_proxy localhost:3001
#   }
#
# Commands (run from repo root or any directory):
#   sudo bash scripts/run-deploywerk-for-caddy.sh run [--api-port 8080] [--http-port 3001] \
#       [--env-file /etc/deploywerk/deploywerk.env] [--web-root /var/www/deploywerk]
#
#   sudo bash scripts/run-deploywerk-for-caddy.sh run --prompt-db --http-port 3001 --api-port 8080
#       # Prompts for Postgres credentials; overrides DATABASE_URL for this run only (fix env file for production).
#
#   bash scripts/run-deploywerk-for-caddy.sh caddy-snippet [--domain deploywerk.orbytals.com] [--http-port 3001]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

die() { echo "error: $*" >&2; exit 1; }

usage() {
  cat <<'EOF'
Commands:  run | caddy-snippet | help

How the app is started (command "run"):
  1) Source --env-file (default /etc/deploywerk/deploywerk.env) for secrets and settings.
  2) Optionally (--prompt-db) ask for Postgres credentials and export DATABASE_URL.
  3) Force HOST=127.0.0.1 and PORT=--api-port, then start deploywerk-api in the background.
  4) Start nginx in the foreground (daemon off) so Ctrl+C stops nginx; the script then stops the API.

run flags:
  --api-port N       API (default 8080)
  --http-port N      nginx loopback (default 3001)
  --env-file PATH
  --web-root PATH    SPA root (default /var/www/deploywerk; must contain index.html)
  --workdir PATH     cd before API (default /opt/deploywerk if present, else repo root)
  --proto STR        X-Forwarded-Proto to API (default https)
  --bind ADDR        nginx listen IP (default 127.0.0.1)
  --extra-listen A   extra nginx listen IP (e.g. 172.17.0.1 for Docker→host)
  --prompt-db        Prompt for Postgres connection (overrides DATABASE_URL for this process tree)

Env: DEPLOYWERK_API_BIN, DEPLOYWERK_* mirror the flags above.

Requires: nginx, deploywerk-api (PATH, DEPLOYWERK_API_BIN, or target/release).
EOF
}

API_PORT="${DEPLOYWERK_API_PORT:-8080}"
HTTP_PORT="${DEPLOYWERK_HTTP_PORT:-3001}"
ENV_FILE="${DEPLOYWERK_ENV_FILE:-/etc/deploywerk/deploywerk.env}"
WEB_ROOT="${DEPLOYWERK_WEB_ROOT:-/var/www/deploywerk}"
WORKDIR="${DEPLOYWERK_WORKDIR:-}"
XFP="${DEPLOYWERK_X_FORWARDED_PROTO:-https}"
HTTP_BIND="${DEPLOYWERK_HTTP_BIND:-127.0.0.1}"
EXTRA_LISTENS=()
SNIP_DOMAIN="${DEPLOYWERK_CADDY_DOMAIN:-deploywerk.orbytals.com}"
PROMPT_DB=0

resolve_workdir() {
  [[ -n "$WORKDIR" ]] && { echo "$WORKDIR"; return; }
  [[ -d /opt/deploywerk ]] && { echo /opt/deploywerk; return; }
  echo "$REPO_ROOT"
}

resolve_api_bin() {
  if [[ -n "${DEPLOYWERK_API_BIN:-}" ]]; then
    [[ -x "$DEPLOYWERK_API_BIN" ]] || die "DEPLOYWERK_API_BIN not executable: $DEPLOYWERK_API_BIN"
    echo "$DEPLOYWERK_API_BIN"; return
  fi
  command -v deploywerk-api >/dev/null 2>&1 && { command -v deploywerk-api; return; }
  local rel="$REPO_ROOT/target/release/deploywerk-api"
  [[ -x "$rel" ]] && { echo "$rel"; return; }
  die "deploywerk-api not found (PATH, DEPLOYWERK_API_BIN, or cargo build --release -p deploywerk-api)"
}

require_nginx() { command -v nginx >/dev/null 2>&1 || die "nginx not in PATH"; }

urlencode_component() {
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1], safe=''))" "$1"
    return
  fi
  # Fallback: assume no special characters needing encoding
  echo "$1"
}

prompt_database_url() {
  echo "Interactive Postgres (sets DATABASE_URL for this run; not written to disk)." >&2
  local host port dbname user pass
  read -r -p "Postgres host [127.0.0.1]: " host
  host="${host:-127.0.0.1}"
  read -r -p "Postgres port [5432]: " port
  port="${port:-5432}"
  read -r -p "Database name [deploywerk]: " dbname
  dbname="${dbname:-deploywerk}"
  read -r -p "Database user [orbytals]: " user
  user="${user:-orbytals}"
  read -r -s -p "Database password: " pass
  echo "" >&2
  [[ -n "$pass" ]] || die "password cannot be empty"
  local eu ep
  eu="$(urlencode_component "$user")"
  ep="$(urlencode_component "$pass")"
  export DATABASE_URL="postgresql://${eu}:${ep}@${host}:${port}/${dbname}"
  echo "DATABASE_URL set (user=${user} host=${host} port=${port} db=${dbname})." >&2
}

mime_include_line() {
  local f="/etc/nginx/mime.types"
  if [[ -f "$f" ]]; then echo "    include $f;"
  else cat <<'MIME'
    types {
        text/html html htm shtml;
        text/css css;
        application/javascript js mjs;
        application/json json;
        image/svg+xml svg svgz;
        font/woff woff;
        font/woff2 woff2;
    }
MIME
  fi
}

write_nginx_conf() {
  local out="$1" listen_lines=""
  listen_lines+="    listen ${HTTP_BIND}:${HTTP_PORT};"$'\n'
  local x
  for x in "${EXTRA_LISTENS[@]}"; do
    [[ -n "$x" ]] && listen_lines+="    listen ${x}:${HTTP_PORT};"$'\n'
  done

  cat >"$out" <<NGX
worker_processes 1;
error_log stderr warn;
pid nginx.pid;
events { worker_connections 1024; }
http {
$(mime_include_line)
    default_type application/octet-stream;
    access_log off;
    sendfile on;
    server {
${listen_lines}        server_name _;
        root ${WEB_ROOT};
        index index.html;

        location /api/ {
            proxy_pass http://127.0.0.1:${API_PORT};
            proxy_http_version 1.1;
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto ${XFP};
            proxy_set_header X-Forwarded-Host \$host;
            proxy_buffering off;
            proxy_read_timeout 86400s;
        }

        location / {
            try_files \$uri \$uri/ /index.html;
        }
    }
}
NGX
}

API_PID=""
cleanup() {
  if [[ -n "${API_PID:-}" ]] && kill -0 "$API_PID" 2>/dev/null; then
    kill "$API_PID" 2>/dev/null || true
    wait "$API_PID" 2>/dev/null || true
  fi
}

cmd_run() {
  require_nginx
  local api_bin workdir env_path prefix
  api_bin="$(resolve_api_bin)"
  workdir="$(resolve_workdir)"
  env_path="$ENV_FILE"

  [[ -f "$env_path" ]] || die "env file not found: $env_path"
  [[ -d "$WEB_ROOT" ]] || die "web root not found: $WEB_ROOT"
  [[ -f "$WEB_ROOT/index.html" ]] || die "missing $WEB_ROOT/index.html (nginx needs the built SPA). Example: cd \"$workdir/web\" && npm ci && npm run build && sudo mkdir -p \"$WEB_ROOT\" && sudo cp -r dist/. \"$WEB_ROOT/\""
  [[ -d "$workdir" ]] || die "workdir not found: $workdir"

  prefix="$(mktemp -d "${TMPDIR:-/tmp}/deploywerk-nginx.XXXXXX")"
  write_nginx_conf "$prefix/nginx.conf"

  set -a
  # shellcheck source=/dev/null
  source "$env_path" || die "failed to source env file"
  set +a

  if [[ "$PROMPT_DB" -eq 1 ]]; then
    prompt_database_url
  fi

  export HOST="127.0.0.1"
  export PORT="$API_PORT"

  trap cleanup INT TERM EXIT

  # --- Start deploywerk-api (Rust application server) ---
  echo "Starting deploywerk-api: $api_bin HOST=$HOST PORT=$PORT workdir=$workdir"
  ( cd "$workdir" && exec "$api_bin" ) &
  API_PID=$!
  sleep 0.3
  if ! kill -0 "$API_PID" 2>/dev/null; then
    echo "" >&2
    echo "deploywerk-api exited immediately. Common cause: PostgreSQL rejected DATABASE_URL (wrong password or user)." >&2
    echo "  1) Test:  psql \"\$DATABASE_URL\" -c 'select 1'   (same shell: re-source $env_path or export DATABASE_URL)" >&2
    echo "  2) Fix password in Postgres to match the env file, or update DATABASE_URL (URL-encode special chars in the password)." >&2
    echo "  3) One-run override: add --prompt-db to this command (same --http-port / --api-port)." >&2
    die "deploywerk-api failed to stay running"
  fi

  # --- Start nginx (static SPA + /api reverse proxy to the API) ---
  echo "Starting nginx: prefix=$prefix listen ${HTTP_BIND}:${HTTP_PORT} (+extras) → API 127.0.0.1:${API_PORT}"
  nginx -p "$prefix" -c nginx.conf -g "daemon off;"
}

cmd_caddy_snippet() {
  cat <<EOF
${SNIP_DOMAIN} {
    reverse_proxy localhost:${HTTP_PORT}
}
EOF
}

parse_run_flags() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --api-port) API_PORT="${2:-}"; shift 2 || die "--api-port value" ;;
      --http-port) HTTP_PORT="${2:-}"; shift 2 || die "--http-port value" ;;
      --env-file) ENV_FILE="${2:-}"; shift 2 || die "--env-file path" ;;
      --web-root) WEB_ROOT="${2:-}"; shift 2 || die "--web-root path" ;;
      --workdir) WORKDIR="${2:-}"; shift 2 || die "--workdir path" ;;
      --proto) XFP="${2:-}"; shift 2 || die "--proto value" ;;
      --bind) HTTP_BIND="${2:-}"; shift 2 || die "--bind address" ;;
      --extra-listen) EXTRA_LISTENS+=("${2:-}"); shift 2 || die "--extra-listen address" ;;
      --prompt-db) PROMPT_DB=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) die "unknown flag: $1" ;;
    esac
  done
}

parse_snippet_flags() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --domain) SNIP_DOMAIN="${2:-}"; shift 2 || die "--domain value" ;;
      --http-port) HTTP_PORT="${2:-}"; shift 2 || die "--http-port value" ;;
      -h|--help) usage; exit 0 ;;
      *) die "unknown flag: $1" ;;
    esac
  done
}

main() {
  local sub="${1:-run}"
  shift || true
  case "$sub" in
    help|-h|--help) usage ;;
    run) parse_run_flags "$@"; cmd_run ;;
    caddy-snippet) parse_snippet_flags "$@"; cmd_caddy_snippet ;;
    *) die "unknown command: $sub" ;;
  esac
}

main "$@"
