#!/usr/bin/env bash
# DeployWerk API + loopback nginx for Caddy (or similar reverse proxies).
# Public HTTPS / Let's Encrypt: terminate TLS on Caddy; this stack listens on loopback HTTP only.
#
# Example Caddy site (global ACME/email is separate):
#   deploywerk.example.com {
#       reverse_proxy localhost:3001
#   }
#
# Usage (repo root or any directory):
#   sudo bash scripts/deploywerk-caddy.sh start [--api-port 8080] [--http-port 3001] ...
#   sudo bash scripts/deploywerk-caddy.sh redeploy [--clean] [--build-web] [--http-port 3001] [--api-port 8080] ...
#       # stop → optional cargo clean → cargo build --release → optional npm build + copy dist → start
#   sudo bash scripts/deploywerk-caddy.sh stop
#   sudo bash scripts/deploywerk-caddy.sh clean [--http-port 3001] [--api-port 8080] [--remove-tmp-nginx]
#   sudo bash scripts/deploywerk-caddy.sh status
#   sudo bash scripts/deploywerk-caddy.sh run ...    # foreground (nginx daemon off); Ctrl+C stops all
#   bash scripts/deploywerk-caddy.sh caddy-snippet [--domain ...] [--http-port 3001]
#
# --prompt-db defaults: DEPLOYWERK_PG_PROMPT_PORT (default 15433), DEPLOYWERK_PG_PROMPT_USER (default orbytals).
#
# Privilege / user: running under sudo or as root is supported and common on small servers; the script does not
# require switching away from root. What matters is a writable STATE_DIR and WEB_ROOT and readable --env-file.
# For production hardening, use a dedicated deploywerk user, chown /opt/deploywerk and state paths, and run the
# API under systemd with User=deploywerk — no change to this script is required to adopt that model.
#
# Database URL (see .env.example): repo docker compose publishes Postgres on host port
#   DEPLOYWERK_POSTGRES_HOST_PORT (default 15433), e.g.
#   DATABASE_URL=postgresql://deploywerk:deploywerk@127.0.0.1:15433/deploywerk
#   Native PostgreSQL on the host usually uses 127.0.0.1:5432. Match DATABASE_URL to the port you use.
#
# Demo seeding / sample data (optional, in --env-file): SEED_DEMO_USERS=true creates demo org/team/project
#   (slugs demo, sample), apps hello/api, and RBAC sample users. DEMO_LOGINS_PUBLIC=true lists passwords on the
#   login page — avoid on public production. When seeded, sample logins include:
#   owner@demo.deploywerk.local / DemoOwner1!   admin@demo.deploywerk.local / DemoAdmin1!
#   member@demo.deploywerk.local / DemoMember1!   orgadmin@demo.deploywerk.local / DemoOrgAdmin1!
#   teamadmin@demo.deploywerk.local / DemoTeamAdmin1!   appadmin@demo.deploywerk.local / DemoAppAdmin1!

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

die() { echo "error: $*" >&2; exit 1; }

usage() {
  cat <<'EOF'
Commands:  start | stop | clean | restart | redeploy | status | run | caddy-snippet | help

start (background):
  1) Source --env-file (default /etc/deploywerk/deploywerk.env).
  2) Optionally (--prompt-db) set DATABASE_URL for this run only.
  3) Start deploywerk-api in the background; wait until /api/v1/health responds on the API port (migrations may delay this).
  4) Start nginx (daemon on); wait until the same path works via loopback HTTP_PORT; write PID files under DEPLOYWERK_STATE_DIR.

stop:  Stop nginx then API using saved PIDs; remove state files.

clean:  Full operational teardown for the loopback stack: run stop (continues on partial failure), wait briefly,
  kill any stale listeners on the API and loopback HTTP ports, remove leftover PID/prefix state files under
  STATE_DIR, and optionally (--remove-tmp-nginx) delete /tmp/deploywerk-nginx.* temp prefix dirs created by
  this script. Prints next steps for DATABASE_URL and start.

restart:  stop then start (passes through run flags).

redeploy:  stop (ok if already stopped), optional "cargo clean", "cargo build --release" for deploywerk-api,
  optional "npm ci && npm run build" in web/ and copy dist/ to --web-root, then start.
  Example:  sudo bash scripts/deploywerk-caddy.sh redeploy --clean --build-web --http-port 3001 --api-port 8080
  Example (interactive DATABASE_URL, not written to disk):  ... redeploy --prompt-db --build-web ...
  After build, the started API binary is workdir/target/release/deploywerk-api (see DEPLOYWERK_API_BIN).

status:  Show whether API and nginx processes from state files are alive.

run (foreground):  Same as the former run-deploywerk-for-caddy script — API in background, nginx
  in the foreground (daemon off) so Ctrl+C stops nginx and the script stops the API.

Shared flags:
  --api-port N       API (default 8080)
  --http-port N      nginx loopback (default 3001)
  --env-file PATH
  --web-root PATH    SPA root (default /var/www/deploywerk; must contain index.html)
  --workdir PATH     cd before API (default /opt/deploywerk if present, else repo root)
  --proto STR        X-Forwarded-Proto to API (default https)
  --bind ADDR        nginx listen IP (default 127.0.0.1)
  --extra-listen A   extra nginx listen IP (e.g. 172.17.0.1 for Docker→host)
  --prompt-db        Prompt for Postgres (overrides DATABASE_URL for this process tree)

clean-only flags:
  --remove-tmp-nginx  Remove top-level ${TMPDIR:-/tmp}/deploywerk-nginx.* directories (this script's mktemp prefix only)

redeploy-only flags:
  --clean            Run "cargo clean" in workdir before building the API
  --build-web        Run "npm ci && npm run build" in workdir/web, then copy dist/ to --web-root

Env: DEPLOYWERK_STATE_DIR (default /var/lib/deploywerk/run), DEPLOYWERK_API_BIN, DEPLOYWERK_* mirror flags.
     DEPLOYWERK_PG_PROMPT_PORT — default port shown for --prompt-db (default 15433; matches Compose default).
     DEPLOYWERK_PG_PROMPT_USER — default DB user shown for --prompt-db (default orbytals).
     DEPLOYWERK_CARGO / DEPLOYWERK_NPM — full paths if cargo/npm not on PATH (common with sudo).
     DEPLOYWERK_SKIP_PSQL_VERIFY=1 — skip optional psql check before starting the API.
     DEPLOYWERK_SKIP_PORT_CHECK=1 — skip \"port already in use\" preflight (not recommended).
     DEPLOYWERK_HEALTH_WAIT_SECS — max seconds to wait for /api/v1/health after API and after nginx (default 120).
     DEPLOYWERK_SKIP_HEALTH_WAIT=1 — skip those checks (not recommended; risk of 502 until migrations finish).

Database / Docker Postgres: DEPLOYWERK_POSTGRES_HOST_PORT (default 15433) must match DATABASE_URL in --env-file
  when using repo docker compose postgres (host uses 127.0.0.1:that_port; native install often uses 5432).

Demo data (env file): SEED_DEMO_USERS=true seeds demo org/team/project and users; DEMO_LOGINS_PUBLIC=true
  exposes demo passwords on the login page (avoid on public production). Sample logins after seed:
  owner@demo.deploywerk.local / DemoOwner1!  admin@demo.deploywerk.local / DemoAdmin1!  member@demo.deploywerk.local / DemoMember1!
  orgadmin@demo.deploywerk.local / DemoOrgAdmin1!  teamadmin@demo.deploywerk.local / DemoTeamAdmin1!  appadmin@demo.deploywerk.local / DemoAppAdmin1!

Requires: nginx, deploywerk-api (PATH, DEPLOYWERK_API_BIN, or target/release). redeploy also needs cargo;
  --build-web needs npm/node.

  Root vs dedicated user: sudo/root is fine for this script; production setups often use a deploywerk system
  user, chown application dirs, and systemd User=deploywerk — optional hardening, not required for the script to run.

  Under sudo, PATH often omits rustup (~/.cargo/bin). Use DEPLOYWERK_CARGO=/root/.cargo/bin/cargo or:
    sudo env "PATH=/root/.cargo/bin:${PATH}" bash scripts/deploywerk-caddy.sh redeploy ...
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
STATE_DIR="${DEPLOYWERK_STATE_DIR:-/var/lib/deploywerk/run}"
PG_PROMPT_PORT="${DEPLOYWERK_PG_PROMPT_PORT:-15433}"
PG_PROMPT_USER="${DEPLOYWERK_PG_PROMPT_USER:-orbytals}"
REDEPLOY_CLEAN=0
REDEPLOY_BUILD_WEB=0
CLEAN_REMOVE_TMP_NGINX=0

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

# sudo resets PATH; rustup usually installs cargo under ~/.cargo/bin (e.g. /root/.cargo/bin).
resolve_cargo() {
  local c
  if [[ -n "${DEPLOYWERK_CARGO:-}" ]]; then
    [[ -x "$DEPLOYWERK_CARGO" ]] || die "DEPLOYWERK_CARGO not executable: $DEPLOYWERK_CARGO"
    echo "$DEPLOYWERK_CARGO"
    return
  fi
  c="$(command -v cargo 2>/dev/null || true)"
  [[ -n "$c" && -x "$c" ]] && { echo "$c"; return; }
  for c in /root/.cargo/bin/cargo "$HOME/.cargo/bin/cargo" /usr/local/cargo/bin/cargo; do
    [[ -x "$c" ]] && { echo "$c"; return; }
  done
  echo "cargo not found. Rust/cargo is required for redeploy." >&2
  echo "  Install: https://rustup.rs/  (as the user that builds, often root on servers: curl ... | sh -s -- -y)" >&2
  echo "  Or point to the binary:  export DEPLOYWERK_CARGO=/root/.cargo/bin/cargo" >&2
  echo "  Or preserve PATH under sudo:  sudo env \"PATH=\$HOME/.cargo/bin:\$PATH\" bash $0 redeploy ..." >&2
  die "cargo not in PATH"
}

resolve_npm() {
  local n f
  if [[ -n "${DEPLOYWERK_NPM:-}" ]]; then
    [[ -x "$DEPLOYWERK_NPM" ]] || die "DEPLOYWERK_NPM not executable: $DEPLOYWERK_NPM"
    echo "$DEPLOYWERK_NPM"
    return
  fi
  n="$(command -v npm 2>/dev/null || true)"
  [[ -n "$n" && -x "$n" ]] && { echo "$n"; return; }
  for n in /usr/bin/npm /usr/local/bin/npm; do
    [[ -x "$n" ]] && { echo "$n"; return; }
  done
  shopt -s nullglob
  for f in /root/.nvm/versions/node/*/bin/npm; do
    [[ -x "$f" ]] && { echo "$f"; shopt -u nullglob; return; }
  done
  shopt -u nullglob
  echo "npm not found (--build-web). Install Node.js or set DEPLOYWERK_NPM." >&2
  echo "  Under sudo try: sudo env \"PATH=/usr/bin:\$PATH\" bash ...  or  sudo env \"PATH=\$HOME/.nvm/versions/node/*/bin:\$PATH\" ..." >&2
  die "npm not in PATH"
}

require_nginx() { command -v nginx >/dev/null 2>&1 || die "nginx not in PATH"; }

urlencode_component() {
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1], safe=''))" "$1"
    return
  fi
  echo "$1"
}

verify_database_url() {
  [[ "${DEPLOYWERK_SKIP_PSQL_VERIFY:-}" == "1" ]] && return 0
  [[ -n "${DATABASE_URL:-}" ]] || return 0
  command -v psql >/dev/null 2>&1 || {
    echo "Tip: install \`postgresql-client\` (\`psql\`) to verify DATABASE_URL before starting the API." >&2
    return 0
  }
  echo "Verifying DATABASE_URL with psql..." >&2
  if psql "$DATABASE_URL" -c 'select 1' >/dev/null 2>&1; then
    echo "Postgres connection OK." >&2
    return 0
  fi
  echo "psql could not connect: wrong password, wrong user, or database does not allow this login." >&2
  echo "  Your Postgres role must exist and the password must match (e.g. Docker Postgres often uses user \`deploywerk\`, not \`orbytals\`)." >&2
  echo "  Test:  psql \"\$DATABASE_URL\" -c 'select 1'" >&2
  echo "  As superuser:  sudo -u postgres psql -c \"\\du\"   # list roles" >&2
  die "DATABASE_URL verification failed"
}

# --- TCP port helpers (deploywerk-api + loopback nginx HTTP_PORT) -----------------

list_tcp_listener_pids_on_port() {
  local port="$1"
  if command -v lsof >/dev/null 2>&1; then
    lsof -tiTCP:"$port" -sTCP:LISTEN 2>/dev/null
    return 0
  fi
  if command -v ss >/dev/null 2>&1; then
    ss -ltnp 2>/dev/null | grep -E ":${port}[[:space:]]" | sed -n 's/.*pid=\([0-9]*\).*/\1/p' | sort -u
  fi
}

show_tcp_port_listeners() {
  local port="$1"
  echo "Processes listening on TCP port ${port}:" >&2
  if command -v ss >/dev/null 2>&1; then
    ss -ltnp 2>/dev/null | grep -E ":${port}[[:space:]]" >&2 || true
  fi
  if command -v lsof >/dev/null 2>&1; then
    lsof -iTCP:"$port" -sTCP:LISTEN -n -P 2>/dev/null >&2 || true
  fi
}

tcp_port_busy() {
  local port="$1"
  [[ -n "$(list_tcp_listener_pids_on_port "$port" | tr -d '[:space:]')" ]]
}

require_loopback_api_port_free() {
  [[ "${DEPLOYWERK_SKIP_PORT_CHECK:-}" == "1" ]] && return 0
  local port="$1"
  tcp_port_busy "$port" || return 0
  echo "" >&2
  echo "Refusing to start: TCP port ${port} is already in use (Linux: os error 98 = Address already in use)." >&2
  show_tcp_port_listeners "$port"
  echo "  Run:  sudo bash \"$0\" stop --api-port ${API_PORT} --http-port ${HTTP_PORT}  (match your start flags)" >&2
  echo "  \`stop\` kills stale deploywerk-api on this port; or kill the PID shown above." >&2
  die "port ${port} busy"
}

require_loopback_http_port_free() {
  [[ "${DEPLOYWERK_SKIP_PORT_CHECK:-}" == "1" ]] && return 0
  local port="$1"
  tcp_port_busy "$port" || return 0
  echo "" >&2
  echo "Refusing to start nginx: loopback HTTP port ${port} is already in use (Caddy reverse_proxy target)." >&2
  show_tcp_port_listeners "$port"
  echo "  Run:  sudo bash \"$0\" stop --http-port ${port} --api-port ${API_PORT}  (match your start flags)" >&2
  echo "  \`stop\` kills stale nginx on this port; or: sudo fuser -k ${port}/tcp  (only if nothing else should use this port)" >&2
  die "http port ${port} busy"
}

kill_stale_deploywerk_api_on_port() {
  local port="$1" pid
  [[ -n "$port" ]] || return 0
  while IFS= read -r pid; do
    [[ -z "$pid" ]] && continue
    [[ -r "/proc/$pid/comm" ]] || continue
    if grep -qx 'deploywerk-api' "/proc/$pid/comm" 2>/dev/null; then
      echo "Stopping stale deploywerk-api (pid $pid) on port $port" >&2
      kill "$pid" 2>/dev/null || true
      sleep 0.3
      kill -9 "$pid" 2>/dev/null || true
    fi
  done < <(list_tcp_listener_pids_on_port "$port")
}

kill_stale_nginx_on_port() {
  local port="$1" round pid c found
  [[ -n "$port" ]] || return 0
  for round in 1 2 3; do
    found=0
    while IFS= read -r pid; do
      [[ -z "$pid" ]] && continue
      [[ -r "/proc/$pid/comm" ]] || continue
      c="$(tr -d '\0' <"/proc/$pid/comm" 2>/dev/null || true)"
      case "$c" in
        nginx*)
          echo "Stopping nginx (pid $pid) still listening on port ${port}" >&2
          kill "$pid" 2>/dev/null || true
          found=1
          ;;
      esac
    done < <(list_tcp_listener_pids_on_port "$port")
    [[ "$found" -eq 0 ]] && break
    sleep 0.4
  done
  while IFS= read -r pid; do
    [[ -z "$pid" ]] && continue
    [[ -r "/proc/$pid/comm" ]] || continue
    c="$(tr -d '\0' <"/proc/$pid/comm" 2>/dev/null || true)"
    case "$c" in
      nginx*)
        echo "Force-stopping nginx (pid $pid) on port ${port}" >&2
        kill -9 "$pid" 2>/dev/null || true
        ;;
    esac
  done < <(list_tcp_listener_pids_on_port "$port")
  sleep 0.2
}

nginx_start_failed_help() {
  local prefix="$1"
  echo "" >&2
  echo "nginx failed to bind (often port ${HTTP_PORT} already in use — stale loopback nginx from a prior run)." >&2
  kill_stale_nginx_on_port "$HTTP_PORT"
  show_tcp_port_listeners "$HTTP_PORT"
  echo "  Try:  sudo bash \"$0\" stop --http-port ${HTTP_PORT} --api-port ${API_PORT}" >&2
  rm -rf "$prefix" 2>/dev/null || true
}

api_exited_help() {
  local env_path="$1"
  echo "" >&2
  echo "deploywerk-api exited immediately. Common causes:" >&2
  echo "  - Port ${API_PORT} already in use (bind fails). Check:  ss -ltnp | grep :${API_PORT}   or   lsof -i :${API_PORT}" >&2
  echo "  - PostgreSQL rejected DATABASE_URL (wrong password / user)." >&2
  echo "  - Test DB:  psql \"\$DATABASE_URL\" -c 'select 1'   (re-source ${env_path} if needed)" >&2
  echo "  - One-run DB override: add --prompt-db to start|run|restart|redeploy (same --api-port / --http-port)." >&2
  echo "  - Free the API port:  sudo bash \"$0\" stop --api-port ${API_PORT}" >&2
}

# GET url; succeeds on HTTP 2xx. Requires curl or wget unless health wait is skipped.
http_get_silent() {
  local url="$1"
  if command -v curl >/dev/null 2>&1; then
    curl -sf --max-time 5 "$url" >/dev/null
    return $?
  fi
  if command -v wget >/dev/null 2>&1; then
    wget -q -O /dev/null --timeout=5 "$url"
    return $?
  fi
  return 1
}

health_wait_failed_help() {
  local url="$1" which="$2"
  echo "" >&2
  echo "Health check failed: ${which} did not respond OK at ${url} within ${DEPLOYWERK_HEALTH_WAIT_SECS:-120}s." >&2
  echo "  - First boot can be slow (migrations). Increase:  DEPLOYWERK_HEALTH_WAIT_SECS=300" >&2
  echo "  - Verify DB:  psql \"\$DATABASE_URL\" -c 'select 1'   and API logs / journal." >&2
  echo "  - Manual check:  curl -sS \"${url}\"" >&2
}

# Poll until HTTP 2xx or timeout. Skipped when DEPLOYWERK_SKIP_HEALTH_WAIT=1.
wait_for_http_ok() {
  [[ "${DEPLOYWERK_SKIP_HEALTH_WAIT:-}" == "1" ]] && {
    echo "warning: DEPLOYWERK_SKIP_HEALTH_WAIT=1 — not verifying /api/v1/health (502 possible until API is ready)." >&2
    return 0
  }
  command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 || \
    die "install curl or wget for start-time health checks (or set DEPLOYWERK_SKIP_HEALTH_WAIT=1)"
  local url="$1" label="$2"
  local max_secs="${DEPLOYWERK_HEALTH_WAIT_SECS:-120}"
  local start elapsed=0
  start="$(date +%s)"
  while true; do
    if http_get_silent "$url"; then
      [[ "$elapsed" -gt 2 ]] && echo "${label}: responding at ${url}" >&2
      return 0
    fi
    elapsed=$(( $(date +%s) - start ))
    if [[ "$elapsed" -ge "$max_secs" ]]; then
      return 1
    fi
    if (( elapsed > 0 && elapsed % 15 == 0 )); then
      echo "waiting for ${label} (${elapsed}s / ${max_secs}s): ${url}" >&2
    fi
    sleep 1
  done
}

abort_start_after_api_health_fail() {
  local apid="$1" prefix="$2"
  kill "$apid" 2>/dev/null || true
  wait "$apid" 2>/dev/null || true
  rm -f "$API_PID_FILE"
  rm -rf "$prefix"
  health_wait_failed_help "http://127.0.0.1:${API_PORT}/api/v1/health" "deploywerk-api"
  die "deploywerk-api did not become healthy in time"
}

abort_start_after_nginx_health_fail() {
  local apid="$1" prefix="$2"
  nginx -p "$prefix" -c nginx.conf -s quit 2>/dev/null || true
  sleep 0.2
  if [[ -f "$prefix/nginx.pid" ]]; then
    local npid
    npid="$(cat "$prefix/nginx.pid")"
    [[ -n "$npid" ]] && kill "$npid" 2>/dev/null || true
    sleep 0.1
    [[ -n "$npid" ]] && kill -9 "$npid" 2>/dev/null || true
  fi
  kill "$apid" 2>/dev/null || true
  wait "$apid" 2>/dev/null || true
  rm -rf "$prefix" 2>/dev/null || true
  rm -f "$API_PID_FILE" "$NGINX_PID_FILE" "$NGINX_PREFIX_FILE"
  health_wait_failed_help "http://127.0.0.1:${HTTP_PORT}/api/v1/health" "nginx→API"
  die "loopback nginx did not proxy /api/v1/health in time"
}

prompt_database_url() {
  if ! command -v python3 >/dev/null 2>&1; then
    echo "warning: python3 not found; passwords with @ # : / etc. may need manual URL-encoding in DATABASE_URL." >&2
  fi
  echo "Interactive Postgres (sets DATABASE_URL for this run; not written to disk)." >&2
  local host port dbname user pass
  read -r -p "Postgres host [127.0.0.1]: " host
  host="${host:-127.0.0.1}"
  read -r -p "Postgres port [${PG_PROMPT_PORT}]: " port
  port="${port:-$PG_PROMPT_PORT}"
  read -r -p "Database name [deploywerk]: " dbname
  dbname="${dbname:-deploywerk}"
  read -r -p "Database user [${PG_PROMPT_USER}]: " user
  user="${user:-$PG_PROMPT_USER}"
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
            proxy_connect_timeout 10s;
            proxy_send_timeout 60s;
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

state_paths() {
  API_PID_FILE="${STATE_DIR}/deploywerk-api.pid"
  NGINX_PID_FILE="${STATE_DIR}/nginx.pid"
  NGINX_PREFIX_FILE="${STATE_DIR}/nginx-prefix.path"
}

api_running() {
  state_paths
  [[ -f "$API_PID_FILE" ]] && kill -0 "$(cat "$API_PID_FILE")" 2>/dev/null
}

nginx_running() {
  state_paths
  [[ -f "$NGINX_PID_FILE" ]] && kill -0 "$(cat "$NGINX_PID_FILE")" 2>/dev/null
}

clear_stale_state() {
  state_paths
  if [[ -f "$API_PID_FILE" ]] && ! kill -0 "$(cat "$API_PID_FILE")" 2>/dev/null; then
    rm -f "$API_PID_FILE"
  fi
  if [[ -f "$NGINX_PID_FILE" ]] && ! kill -0 "$(cat "$NGINX_PID_FILE")" 2>/dev/null; then
    rm -f "$NGINX_PID_FILE" "$NGINX_PREFIX_FILE"
  fi
}

prepare_env_and_paths() {
  require_nginx
  [[ -f "$ENV_FILE" ]] || die "env file not found: $ENV_FILE"
  [[ -d "$WEB_ROOT" ]] || die "web root not found: $WEB_ROOT"
  [[ -f "$WEB_ROOT/index.html" ]] || die "missing $WEB_ROOT/index.html (nginx needs the built SPA). Example: cd \"$(resolve_workdir)/web\" && npm ci && npm run build && sudo mkdir -p \"$WEB_ROOT\" && sudo cp -r dist/. \"$WEB_ROOT/\""
  local wd
  wd="$(resolve_workdir)"
  [[ -d "$wd" ]] || die "workdir not found: $wd"
}

cmd_start() {
  prepare_env_and_paths
  state_paths
  mkdir -p "$STATE_DIR" || die "cannot create $STATE_DIR"
  clear_stale_state

  if api_running || nginx_running; then
    die "already running (use stop or restart). PIDs: ${STATE_DIR}/*.pid"
  fi

  local api_bin workdir env_path prefix
  api_bin="$(resolve_api_bin)"
  workdir="$(resolve_workdir)"
  env_path="$ENV_FILE"

  prefix="$(mktemp -d "${TMPDIR:-/tmp}/deploywerk-nginx.XXXXXX")"
  write_nginx_conf "$prefix/nginx.conf"

  set -a
  # shellcheck source=/dev/null
  source "$env_path" || die "failed to source env file"
  set +a

  if [[ "$PROMPT_DB" -eq 1 ]]; then
    prompt_database_url
  fi

  verify_database_url

  export HOST="127.0.0.1"
  export PORT="$API_PORT"

  kill_stale_deploywerk_api_on_port "$API_PORT"
  sleep 0.2
  require_loopback_api_port_free "$API_PORT"

  echo "Starting deploywerk-api: $api_bin HOST=$HOST PORT=$PORT workdir=$workdir"
  ( cd "$workdir" && exec "$api_bin" ) &
  local apid=$!
  echo "$apid" >"$API_PID_FILE"
  sleep 0.5
  if ! kill -0 "$apid" 2>/dev/null; then
    rm -f "$API_PID_FILE"
    rm -rf "$prefix"
    api_exited_help "$env_path"
    die "deploywerk-api failed to stay running"
  fi

  if ! wait_for_http_ok "http://127.0.0.1:${API_PORT}/api/v1/health" "deploywerk-api"; then
    abort_start_after_api_health_fail "$apid" "$prefix"
  fi

  kill_stale_nginx_on_port "$HTTP_PORT"
  sleep 0.2
  require_loopback_http_port_free "$HTTP_PORT"

  echo "Starting nginx (background): prefix=$prefix listen ${HTTP_BIND}:${HTTP_PORT} → API 127.0.0.1:${API_PORT}"
  if ! nginx -p "$prefix" -c nginx.conf; then
    kill "$apid" 2>/dev/null || true
    wait "$apid" 2>/dev/null || true
    rm -f "$API_PID_FILE"
    nginx_start_failed_help "$prefix"
    die "nginx failed to start"
  fi

  sleep 0.2
  [[ -f "$prefix/nginx.pid" ]] || {
    kill "$apid" 2>/dev/null || true
    rm -f "$API_PID_FILE"
    nginx_start_failed_help "$prefix"
    die "nginx did not write pid file"
  }
  cp "$prefix/nginx.pid" "$NGINX_PID_FILE"
  printf '%s\n' "$prefix" >"$NGINX_PREFIX_FILE"

  if ! wait_for_http_ok "http://127.0.0.1:${HTTP_PORT}/api/v1/health" "nginx→API"; then
    abort_start_after_nginx_health_fail "$apid" "$prefix"
  fi

  echo "DeployWerk started. State: $STATE_DIR"
}

cmd_stop() {
  state_paths
  require_nginx

  if [[ -f "$NGINX_PREFIX_FILE" ]]; then
    local prefix
    prefix="$(cat "$NGINX_PREFIX_FILE")"
    if [[ -n "$prefix" && -d "$prefix" && -f "$prefix/nginx.conf" ]]; then
      nginx -p "$prefix" -c nginx.conf -s quit 2>/dev/null || true
    fi
  fi
  sleep 0.3
  if [[ -f "$NGINX_PID_FILE" ]]; then
    local npid
    npid="$(cat "$NGINX_PID_FILE")"
    if [[ -n "$npid" ]] && kill -0 "$npid" 2>/dev/null; then
      kill "$npid" 2>/dev/null || true
      sleep 0.2
      kill -9 "$npid" 2>/dev/null || true
    fi
  fi

  if [[ -f "$API_PID_FILE" ]]; then
    local apid
    apid="$(cat "$API_PID_FILE")"
    if [[ -n "$apid" ]] && kill -0 "$apid" 2>/dev/null; then
      kill "$apid" 2>/dev/null || true
      wait "$apid" 2>/dev/null || true
    fi
  fi
  kill_stale_deploywerk_api_on_port "$API_PORT"
  sleep 0.2
  kill_stale_nginx_on_port "$HTTP_PORT"
  sleep 0.2

  if [[ -f "$NGINX_PREFIX_FILE" ]]; then
    local pfx
    pfx="$(cat "$NGINX_PREFIX_FILE")"
    [[ -n "$pfx" && -d "$pfx" ]] && rm -rf "$pfx" 2>/dev/null || true
  fi
  rm -f "$API_PID_FILE" "$NGINX_PID_FILE" "$NGINX_PREFIX_FILE"
  echo "DeployWerk stopped."
}

cmd_clean() {
  state_paths
  cmd_stop || true
  sleep 0.5
  kill_stale_deploywerk_api_on_port "$API_PORT"
  kill_stale_nginx_on_port "$HTTP_PORT"
  sleep 0.2
  state_paths
  rm -f "$API_PID_FILE" "$NGINX_PID_FILE" "$NGINX_PREFIX_FILE" 2>/dev/null || true
  if [[ "$CLEAN_REMOVE_TMP_NGINX" -eq 1 ]]; then
    echo "Removing top-level ${TMPDIR:-/tmp}/deploywerk-nginx.* directories (mktemp prefix from this script only)..." >&2
    find "${TMPDIR:-/tmp}" -maxdepth 1 -type d -name 'deploywerk-nginx.*' -exec rm -rf {} + 2>/dev/null || true
  fi
  echo "DeployWerk clean finished." >&2
  echo "Next steps:" >&2
  echo "  1) Set a working DATABASE_URL in ${ENV_FILE} (default for --env-file). Test: psql \"\$DATABASE_URL\" -c 'select 1'" >&2
  echo "  2) sudo bash \"$0\" start --http-port ${HTTP_PORT} --api-port ${API_PORT}" >&2
  echo "     Or: start --prompt-db to set DATABASE_URL for this run only (not written to disk)." >&2
}

cmd_status() {
  state_paths
  clear_stale_state
  echo "State directory: $STATE_DIR"
  if [[ -f "$API_PID_FILE" ]]; then
    local apid
    apid="$(cat "$API_PID_FILE")"
    if kill -0 "$apid" 2>/dev/null; then
      echo "deploywerk-api: running (pid $apid)"
    else
      echo "deploywerk-api: not running (stale pid file cleared)"
    fi
  else
    echo "deploywerk-api: not running"
  fi
  if [[ -f "$NGINX_PID_FILE" ]]; then
    local npid
    npid="$(cat "$NGINX_PID_FILE")"
    if kill -0 "$npid" 2>/dev/null; then
      echo "nginx: running (pid $npid)  loopback http://${HTTP_BIND}:${HTTP_PORT}"
    else
      echo "nginx: not running (stale pid file cleared)"
    fi
  else
    echo "nginx: not running"
  fi
}

API_PID_FG=""
cleanup_fg() {
  if [[ -n "${API_PID_FG:-}" ]] && kill -0 "$API_PID_FG" 2>/dev/null; then
    kill "$API_PID_FG" 2>/dev/null || true
    wait "$API_PID_FG" 2>/dev/null || true
  fi
}

cmd_run() {
  prepare_env_and_paths
  local api_bin workdir env_path prefix
  api_bin="$(resolve_api_bin)"
  workdir="$(resolve_workdir)"
  env_path="$ENV_FILE"

  prefix="$(mktemp -d "${TMPDIR:-/tmp}/deploywerk-nginx.XXXXXX")"
  write_nginx_conf "$prefix/nginx.conf"

  set -a
  # shellcheck source=/dev/null
  source "$env_path" || die "failed to source env file"
  set +a

  if [[ "$PROMPT_DB" -eq 1 ]]; then
    prompt_database_url
  fi

  verify_database_url

  export HOST="127.0.0.1"
  export PORT="$API_PORT"

  kill_stale_deploywerk_api_on_port "$API_PORT"
  sleep 0.2
  require_loopback_api_port_free "$API_PORT"

  trap cleanup_fg INT TERM EXIT

  echo "Starting deploywerk-api: $api_bin HOST=$HOST PORT=$PORT workdir=$workdir"
  ( cd "$workdir" && exec "$api_bin" ) &
  API_PID_FG=$!
  sleep 0.5
  if ! kill -0 "$API_PID_FG" 2>/dev/null; then
    api_exited_help "$env_path"
    die "deploywerk-api failed to stay running"
  fi

  if ! wait_for_http_ok "http://127.0.0.1:${API_PORT}/api/v1/health" "deploywerk-api"; then
    kill "$API_PID_FG" 2>/dev/null || true
    wait "$API_PID_FG" 2>/dev/null || true
    rm -rf "$prefix"
    health_wait_failed_help "http://127.0.0.1:${API_PORT}/api/v1/health" "deploywerk-api"
    die "deploywerk-api did not become healthy in time"
  fi

  kill_stale_nginx_on_port "$HTTP_PORT"
  sleep 0.2
  require_loopback_http_port_free "$HTTP_PORT"

  echo "Starting nginx: prefix=$prefix listen ${HTTP_BIND}:${HTTP_PORT} (+extras) → API 127.0.0.1:${API_PORT}"
  nginx -p "$prefix" -c nginx.conf -g "daemon off;" || {
    nginx_start_failed_help "$prefix"
    die "nginx failed to start"
  }
}

cmd_caddy_snippet() {
  cat <<EOF
${SNIP_DOMAIN} {
    reverse_proxy localhost:${HTTP_PORT}
}
EOF
}

parse_run_flags() {
  CLEAN_REMOVE_TMP_NGINX=0
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
      --state-dir) STATE_DIR="${2:-}"; shift 2 || die "--state-dir path" ;;
      --prompt-db) PROMPT_DB=1; shift ;;
      --remove-tmp-nginx) CLEAN_REMOVE_TMP_NGINX=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) die "unknown flag: $1" ;;
    esac
  done
}

parse_redeploy_flags() {
  REDEPLOY_CLEAN=0
  REDEPLOY_BUILD_WEB=0
  PROMPT_DB=0
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
      --state-dir) STATE_DIR="${2:-}"; shift 2 || die "--state-dir path" ;;
      --prompt-db) PROMPT_DB=1; shift ;;
      --clean) REDEPLOY_CLEAN=1; shift ;;
      --build-web) REDEPLOY_BUILD_WEB=1; shift ;;
      -h|--help) usage; exit 0 ;;
      *) die "unknown flag: $1" ;;
    esac
  done
}

cmd_redeploy() {
  local workdir api_built cargo_bin npm_bin
  workdir="$(resolve_workdir)"
  [[ -f "$workdir/Cargo.toml" ]] || die "no Cargo.toml in workdir: $workdir"

  cmd_stop || true
  sleep 0.5
  kill_stale_deploywerk_api_on_port "$API_PORT"
  kill_stale_nginx_on_port "$HTTP_PORT"
  sleep 0.2

  cargo_bin="$(resolve_cargo)"
  echo "Using cargo: $cargo_bin"

  if [[ "$REDEPLOY_CLEAN" -eq 1 ]]; then
    echo "Running cargo clean in $workdir"
    ( cd "$workdir" && "$cargo_bin" clean )
  fi

  echo "Building deploywerk-api (release) in $workdir"
  ( cd "$workdir" && "$cargo_bin" build --release -p deploywerk-api --bin deploywerk-api )

  api_built="$workdir/target/release/deploywerk-api"
  [[ -x "$api_built" ]] || die "build did not produce executable: $api_built"
  export DEPLOYWERK_API_BIN="$api_built"

  if [[ "$REDEPLOY_BUILD_WEB" -eq 1 ]]; then
    [[ -d "$workdir/web" ]] || die "no web/ directory in $workdir"
    npm_bin="$(resolve_npm)"
    echo "Using npm: $npm_bin"
    echo "Building web UI in $workdir/web"
    ( cd "$workdir/web" && "$npm_bin" ci && "$npm_bin" run build )
    [[ -f "$workdir/web/dist/index.html" ]] || die "web build did not produce dist/index.html"
    echo "Copying $workdir/web/dist/ -> $WEB_ROOT/"
    mkdir -p "$WEB_ROOT" || die "cannot mkdir $WEB_ROOT"
    cp -a "$workdir/web/dist/." "$WEB_ROOT/" || die "copy to WEB_ROOT failed (try sudo or fix ownership of $WEB_ROOT)"
  fi

  if [[ "$PROMPT_DB" -eq 1 ]]; then
    echo "Interactive DATABASE_URL will be requested before start (not written to $ENV_FILE)." >&2
  fi

  cmd_start
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
  local sub="${1:-status}"
  shift || true
  case "$sub" in
    help|-h|--help) usage ;;
    start) parse_run_flags "$@"; cmd_start ;;
    stop) parse_run_flags "$@"; cmd_stop ;;
    clean) parse_run_flags "$@"; cmd_clean ;;
    restart) parse_run_flags "$@"; cmd_stop; cmd_start ;;
    redeploy) parse_redeploy_flags "$@"; cmd_redeploy ;;
    status) parse_run_flags "$@"; cmd_status ;;
    run) parse_run_flags "$@"; cmd_run ;;
    caddy-snippet) parse_snippet_flags "$@"; cmd_caddy_snippet ;;
    *) die "unknown command: $sub (try: help)" ;;
  esac
}

main "$@"
