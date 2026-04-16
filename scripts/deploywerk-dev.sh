#!/usr/bin/env bash
# Usage: ./scripts/deploywerk-dev.sh run|stop|clean [options]
#
# Default (native): Docker runs Postgres + MinIO (+ init) only; API and Vite run on the host.
#   run [--authentik]     — deps in Docker, then cargo + npm run dev (background)
#   stop [--authentik]    — kill host PIDs; docker compose stop
#   clean [--authentik] [--rmi-local] — kill host PIDs; docker compose down -v
#
# Full stack in Docker (optional):
#   run --docker [--authentik]   — docker compose up -d --build (api + web + deps)
#   stop --docker [--authentik]
#   clean --docker [--authentik] [--rmi-local]
#
# Requires: Docker, docker compose, repo-root .env (copy from .env.example).
# Native run also requires: cargo, npm (Rust + Node on PATH).
# Use Git Bash, WSL, or Unix.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PID_FILE="$ROOT/.deploywerk-dev.pids"
MODE_FILE="$ROOT/.deploywerk-dev.mode"
LOG_DIR="$ROOT/.deploywerk-logs"

usage() {
  cat <<'EOF'
Usage: deploywerk-dev.sh <command> [options]

Commands (default = native host API + Vite):
  run [--authentik]              Start Postgres + MinIO in Docker; run API (cargo) + web (Vite) on host.
  run --docker [--authentik]     Start full stack in Docker (api + web + deps), same as before.

  stop [--docker] [--authentik]  Stop processes (native: kill PIDs + compose stop; docker: compose stop).

  clean [--docker] [--authentik] [--rmi-local]
                                 Native: kill PIDs + compose down -v. Docker: compose down -v.
                                 Use same flags as run. --rmi-local removes local images (docker clean).

URLs (native default):
  Web   http://127.0.0.1:5173
  API   http://127.0.0.1:8080
  MinIO http://127.0.0.1:19000 (S3 API on host)

Logs (native): .deploywerk-logs/api.log  .deploywerk-logs/web.log
Logs (docker): docker compose logs -f api web
EOF
}

require_env_file() {
  if [[ ! -f .env ]]; then
    echo "error: .env not found. Copy .env.example to .env in the repo root." >&2
    exit 1
  fi
}

require_native_tooling() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo not found on PATH (required for native ./scripts/deploywerk-dev.sh run)." >&2
    echo "Install Rust, or use: $0 run --docker" >&2
    exit 1
  fi
  if ! command -v npm >/dev/null 2>&1; then
    echo "error: npm not found on PATH (required for native run)." >&2
    exit 1
  fi
}

load_dotenv() {
  set -a
  # shellcheck disable=SC1091
  source "$ROOT/.env"
  set +a
}

wait_for_postgres() {
  local i
  for ((i = 1; i <= 60; i++)); do
    if docker compose exec -T postgres pg_isready -U deploywerk -d deploywerk >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "error: Postgres did not become ready within 60s" >&2
  return 1
}

wait_for_minio_host() {
  local i
  for ((i = 1; i <= 60; i++)); do
    if curl -sf "http://127.0.0.1:19000/minio/health/live" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "error: MinIO on http://127.0.0.1:19000 did not become ready within 60s" >&2
  return 1
}

wait_for_authentik_live() {
  local i
  for ((i = 1; i <= 120; i++)); do
    if curl -sf "http://127.0.0.1:9000/-/health/live/" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "warning: Authentik http://127.0.0.1:9000/-/health/live/ did not respond within 120s (check: docker compose --profile authentik logs authentik-server)" >&2
  return 0
}

wait_for_api_health() {
  local i
  for ((i = 1; i <= 90; i++)); do
    if curl -sf "http://127.0.0.1:8080/api/v1/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "warning: API http://127.0.0.1:8080/api/v1/health did not respond within 90s" >&2
  return 0
}

kill_native_pids() {
  if [[ ! -f "$PID_FILE" ]]; then
    return 0
  fi
  while read -r pid; do
    [[ -z "${pid:-}" ]] && continue
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done <"$PID_FILE"
  rm -f "$PID_FILE"
}

read_mode_authentik() {
  local a=false
  if [[ -f "$MODE_FILE" ]]; then
    if grep -q '^AUTHENTIK=1$' "$MODE_FILE" 2>/dev/null; then
      a=true
    fi
  fi
  echo "$a"
}

read_mode_stack() {
  local mode=native
  if [[ -f "$MODE_FILE" ]] && grep -q '^MODE=docker$' "$MODE_FILE" 2>/dev/null; then
    mode=docker
  fi
  echo "$mode"
}

write_mode_file() {
  local mode="$1"
  local authentik="$2"
  mkdir -p "$LOG_DIR"
  {
    echo "MODE=$mode"
    if [[ "$authentik" == true ]]; then
      echo "AUTHENTIK=1"
    else
      echo "AUTHENTIK=0"
    fi
  } >"$MODE_FILE"
}

cmd_run() {
  local use_docker=false
  local with_authentik=false
  for arg in "$@"; do
    case "$arg" in
      --docker) use_docker=true ;;
      --authentik) with_authentik=true ;;
      *)
        echo "error: unknown option for run: $arg" >&2
        exit 1
        ;;
    esac
  done

  require_env_file

  if [[ "$use_docker" == true ]]; then
    if [[ "$with_authentik" == true ]]; then
      docker compose --profile authentik up -d --build
    else
      docker compose up -d --build
    fi
    wait_for_postgres
    if [[ "$with_authentik" == true ]]; then
      echo "Waiting for Authentik on :9000…"
      wait_for_authentik_live
      echo "Authentik: https://127.0.0.1:9445/ or http://127.0.0.1:9000/if/admin/"
    fi
    echo "Waiting for API health…"
    wait_for_api_health
    write_mode_file docker "$with_authentik"
    echo ""
    echo "DeployWerk stack is up (Docker: api + web + deps)."
    echo "  Web:  http://127.0.0.1:5173"
    echo "  API:  http://127.0.0.1:8080"
    if [[ "$with_authentik" == true ]]; then
      echo "  Authentik: https://127.0.0.1:9445  (HTTP http://127.0.0.1:9000)"
    fi
    echo "Logs: docker compose logs -f api web"
    echo "Stop: $0 stop"
    return 0
  fi

  require_native_tooling

  if [[ "$with_authentik" == true ]]; then
    docker compose --profile authentik up -d postgres minio minio-init \
      authentik-postgresql authentik-redis authentik-server authentik-worker
  else
    docker compose up -d postgres minio minio-init
  fi

  wait_for_postgres
  echo "Waiting for MinIO on :19000…"
  wait_for_minio_host

  if [[ "$with_authentik" == true ]]; then
    echo "Waiting for Authentik on :9000…"
    wait_for_authentik_live
    echo "Authentik: https://127.0.0.1:9445/ or http://127.0.0.1:9000/if/admin/"
  fi

  kill_native_pids
  mkdir -p "$LOG_DIR" "$ROOT/.deploywerk-git-cache" "$ROOT/.deploywerk-volumes"
  rm -f "$PID_FILE"

  load_dotenv
  export DATABASE_URL="postgresql://deploywerk:deploywerk@127.0.0.1:5432/deploywerk"
  export DEPLOYWERK_DEFAULT_STORAGE_ENDPOINT_URL="http://127.0.0.1:19000"
  export DEPLOYWERK_GIT_CACHE_ROOT="${ROOT}/.deploywerk-git-cache"
  export DEPLOYWERK_VOLUMES_ROOT="${ROOT}/.deploywerk-volumes"
  export HOST="${HOST:-127.0.0.1}"
  export PORT="${PORT:-8080}"

  echo "Starting deploywerk-api (cargo)…"
  cd "$ROOT"
  nohup cargo run -p deploywerk-api --bin deploywerk-api >>"$LOG_DIR/api.log" 2>&1 &
  echo $! >>"$PID_FILE"

  echo "Starting Vite (npm run dev)…"
  cd "$ROOT/web"
  nohup npm run dev >>"$LOG_DIR/web.log" 2>&1 &
  echo $! >>"$PID_FILE"
  cd "$ROOT"

  write_mode_file native "$with_authentik"

  echo "Waiting for API health…"
  wait_for_api_health

  echo ""
  echo "DeployWerk is up (native API + Vite; Postgres + MinIO in Docker)."
  echo "  Web:   http://127.0.0.1:5173"
  echo "  API:   http://127.0.0.1:8080"
  echo "  MinIO: http://127.0.0.1:19000"
  if [[ "$with_authentik" == true ]]; then
    echo "  Authentik: https://127.0.0.1:9445  (HTTP http://127.0.0.1:9000)"
  fi
  echo "Logs: tail -f .deploywerk-logs/api.log .deploywerk-logs/web.log"
  echo "Stop: $0 stop${with_authentik:+ --authentik}"
}

cmd_stop() {
  local use_docker=false
  local with_authentik=false
  for arg in "$@"; do
    case "$arg" in
      --docker) use_docker=true ;;
      --authentik) with_authentik=true ;;
      *)
        echo "error: unknown option for stop: $arg" >&2
        exit 1
        ;;
    esac
  done

  if [[ "$use_docker" == false ]] && [[ -f "$MODE_FILE" ]] && [[ "$(read_mode_stack)" == docker ]]; then
    use_docker=true
    if [[ "$with_authentik" == false ]]; then
      with_authentik="$(read_mode_authentik)"
    fi
  fi

  if [[ "$use_docker" == true ]]; then
    if [[ "$with_authentik" == true ]]; then
      docker compose --profile authentik stop
    else
      docker compose stop
    fi
    rm -f "$MODE_FILE"
    echo "Docker Compose services stopped."
    return 0
  fi

  kill_native_pids
  local a
  a="$(read_mode_authentik)"
  if [[ "$with_authentik" == true ]] || [[ "$a" == true ]]; then
    docker compose --profile authentik stop
  else
    docker compose stop
  fi
  rm -f "$MODE_FILE"
  echo "Native processes stopped (if any); Docker deps stopped."
}

cmd_clean() {
  local use_docker=false
  local with_authentik=false
  local rmi_local=false
  for arg in "$@"; do
    case "$arg" in
      --docker) use_docker=true ;;
      --authentik) with_authentik=true ;;
      --rmi-local) rmi_local=true ;;
      *)
        echo "error: unknown option for clean: $arg" >&2
        exit 1
        ;;
    esac
  done

  if [[ "$use_docker" == false ]] && [[ -f "$MODE_FILE" ]] && [[ "$(read_mode_stack)" == docker ]]; then
    use_docker=true
    if [[ "$with_authentik" == false ]]; then
      with_authentik="$(read_mode_authentik)"
    fi
  fi

  local down_args=(down -v --remove-orphans)
  if [[ "$rmi_local" == true ]]; then
    down_args+=(--rmi local)
  fi

  if [[ "$use_docker" == true ]]; then
    if [[ "$with_authentik" == true ]]; then
      docker compose --profile authentik "${down_args[@]}"
    else
      docker compose "${down_args[@]}"
    fi
    rm -f "$MODE_FILE" "$PID_FILE"
    echo "Removed containers and project volumes${rmi_local:+, local images}."
    return 0
  fi

  kill_native_pids
  local a
  a="$(read_mode_authentik)"
  if [[ "$with_authentik" == true ]] || [[ "$a" == true ]]; then
    docker compose --profile authentik "${down_args[@]}"
  else
    docker compose "${down_args[@]}"
  fi
  rm -f "$MODE_FILE"
  echo "Native processes stopped; containers and project volumes removed${rmi_local:+, local images}."
}

main() {
  local cmd="${1:-}"
  shift || true

  case "$cmd" in
    run) cmd_run "$@" ;;
    stop) cmd_stop "$@" ;;
    clean) cmd_clean "$@" ;;
    ""|-h|--help|help) usage ;;
    *)
      echo "error: unknown command: $cmd" >&2
      usage >&2
      exit 1
      ;;
  esac
}

main "$@"
