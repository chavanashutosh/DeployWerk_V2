#!/usr/bin/env bash
# Usage: ./scripts/deploywerk-dev.sh run|stop|clean [options]
#   run [--authentik]  — docker compose up -d --build (Postgres + api + web; + Authentik profile if flag)
#   stop [--authentik] — docker compose stop (same profile as run)
#   clean [--authentik] [--rmi-local] — docker compose down -v --remove-orphans; optional image removal
#
# Requires: Docker, docker compose, and a repo-root .env (copy from .env.example).
# Use Git Bash, WSL, or Unix.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Usage: deploywerk-dev.sh <command> [options]

Commands:
  run [--authentik]       Build (if needed) and start Postgres, API, and web (nginx on port 5173).
  stop [--authentik]      Stop all Compose services for this project (containers kept).
  clean [--authentik] [--rmi-local]
                          Remove containers and named volumes (fresh DB next run).
                          Use the same --authentik as run if you started Authentik.
                          --rmi-local also removes images built by Compose.

Use the same --authentik flag for run / stop / clean so Authentik services and volumes are included.

Logs: docker compose logs -f api web
  (add --profile authentik and service names when using Authentik)

URLs (default run):
  Web   http://127.0.0.1:5173
  API   http://127.0.0.1:8080
EOF
}

require_env_file() {
  if [[ ! -f .env ]]; then
    echo "error: .env not found. Copy .env.example to .env in the repo root." >&2
    exit 1
  fi
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
  echo "warning: API http://127.0.0.1:8080/api/v1/health did not respond within 90s (see: docker compose logs api)" >&2
  return 0
}

cmd_run() {
  local with_authentik=false
  for arg in "$@"; do
    case "$arg" in
      --authentik) with_authentik=true ;;
      *)
        echo "error: unknown option for run: $arg" >&2
        exit 1
        ;;
    esac
  done

  require_env_file

  if [[ "$with_authentik" == true ]]; then
    docker compose --profile authentik up -d --build
  else
    docker compose up -d --build
  fi

  wait_for_postgres

  if [[ "$with_authentik" == true ]]; then
    echo "Waiting for Authentik on :9000…"
    wait_for_authentik_live
    echo "Authentik: http://127.0.0.1:9000/if/admin/ (first visit: create admin user if new volume)"
  fi

  echo "Waiting for API health…"
  wait_for_api_health

  echo ""
  echo "DeployWerk stack is up (Docker)."
  echo "  Web:  http://127.0.0.1:5173"
  echo "  API:  http://127.0.0.1:8080"
  if [[ "$with_authentik" == true ]]; then
    echo "  Authentik: http://127.0.0.1:9000"
  fi
  echo "Logs: docker compose logs -f api web"
  echo "Stop: $0 stop${with_authentik:+ --authentik}"
}

cmd_stop() {
  local with_authentik=false
  for arg in "$@"; do
    case "$arg" in
      --authentik) with_authentik=true ;;
      *)
        echo "error: unknown option for stop: $arg" >&2
        exit 1
        ;;
    esac
  done

  if [[ "$with_authentik" == true ]]; then
    docker compose --profile authentik stop
  else
    docker compose stop
  fi
  echo "Docker Compose services stopped."
}

cmd_clean() {
  local with_authentik=false
  local rmi_local=false
  for arg in "$@"; do
    case "$arg" in
      --authentik) with_authentik=true ;;
      --rmi-local) rmi_local=true ;;
      *)
        echo "error: unknown option for clean: $arg" >&2
        exit 1
        ;;
    esac
  done

  local down_args=(down -v --remove-orphans)
  if [[ "$rmi_local" == true ]]; then
    down_args+=(--rmi local)
  fi

  if [[ "$with_authentik" == true ]]; then
    docker compose --profile authentik "${down_args[@]}"
  else
    docker compose "${down_args[@]}"
  fi
  echo "Removed containers and project volumes${rmi_local:+, local images}."
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
