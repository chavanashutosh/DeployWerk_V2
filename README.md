# DeployWerk V2

Rust **API** (`deploywerk-api`), **CLI** (`deploywerk-cli`), and **Vite + React** web UI. Teams, projects, environments, Docker applications, deploy jobs (SSH or platform Docker), optional Git webhooks, optional OIDC via **Authentik**.

**Implementation** (API behavior, SQL migrations, UI wiring) lives in this repository. The **Enterprise Platform Specification** (intended product surface) is indexed in [docs/README.md](docs/README.md), starting with [docs/spec/00-overview.md](docs/spec/00-overview.md). **Spec vs code status** (done / partial / pending): [docs/STATUS.md](docs/STATUS.md). Local **platform admin** routes and API checks: [docs/ADMIN_DEV.md](docs/ADMIN_DEV.md).

---

## Run everything (PostgreSQL + API + web)

**Requirements:** Docker (Compose v2) for the default flow. Rust and Node are only needed if you develop with **host** `cargo` / Vite (see below).

1. Copy [.env.example](.env.example) to `.env` in the repo root.
2. Start the full stack in Docker (builds images on first run):

**One script (Git Bash / WSL / macOS / Linux):**

```bash
chmod +x scripts/deploywerk-dev.sh
./scripts/deploywerk-dev.sh run              # Postgres + API + web (nginx on :5173)
./scripts/deploywerk-dev.sh run --authentik  # same + Authentik profile (see below)
./scripts/deploywerk-dev.sh stop             # stop containers (pass --authentik if you used it for run)
./scripts/deploywerk-dev.sh clean            # docker compose down -v (removes DB volumes; same --authentik rule)
./scripts/deploywerk-dev.sh clean --rmi-local  # also remove images built by Compose
```

- **Web UI:** http://127.0.0.1:5173 (nginx serves the built SPA and proxies `/api` to the API container).
- **API (direct):** http://127.0.0.1:8080  
- Compose overrides **`DATABASE_URL`** for the `api` service to use the `postgres` container (`@postgres:5432`). Your `.env` may still say `127.0.0.1` for local tools; that is fine for the API container.

Optional: local mail stack (Stalwart + Isotope webmail) is documented in [docs/MAIL_DEV.md](docs/MAIL_DEV.md).
Optional: bare metal install (Ubuntu 24.04 LTS; DeployWerk + optional second host for mail/Matrix/DNS; nginx or Traefik; Let’s Encrypt; XRDP) is documented in [docs/BARE_METAL.md](docs/BARE_METAL.md).

If you started **Authentik** (`run --authentik`), use **`stop --authentik`** and **`clean --authentik`** so Authentik services and volumes are included.

**Windows (PowerShell)** — use Docker Compose directly (same as the script):

```powershell
docker compose up -d --build
# or with Authentik:
docker compose --profile authentik up -d --build
```

Or run **`deploywerk-dev.sh`** from **Git Bash** for the same commands. The Authentik-only helper script remains optional:

```powershell
.\scripts\deploywerk-dev-authentik.ps1
```

### Host development (Postgres in Docker, API + web on the machine)

For fast iteration without rebuilding images:

1. `docker compose up -d postgres` (starts only Postgres; does not start `api` / `web` services).
2. From repo root: `cargo run -p deploywerk-api --bin deploywerk-api`
3. `cd web && npm install && npm run dev` → http://127.0.0.1:5173 (Vite proxies `/api` per [web/vite.config.ts](web/vite.config.ts)).

### Web `/api` returns 404

The browser calls paths like `/api/v1/bootstrap`. Those are **not** served by the static UI—they must reach **`deploywerk-api`**.

| Cause | What to do |
|--------|------------|
| API not running | With the Docker stack, nginx proxies `/api` to the API container; confirm `docker compose ps` and `curl -sf http://127.0.0.1:8080/api/v1/health`. On host dev, start the API on **8080** (or match `DEPLOYWERK_API_PROXY`). |
| `vite preview` or static `dist/` without a proxy | Vite’s **`preview`** server proxies `/api` the same as dev (see [web/vite.config.ts](web/vite.config.ts)), but a generic static host does not. Either put a reverse proxy in front, or set **`VITE_API_URL`** at build time to your API origin (see [.env.example](.env.example)). |
| API on a non-default port | Set **`DEPLOYWERK_API_PROXY`** in `.env` (repo root) to match `http://HOST:PORT` (Vite loads env from the repo root). Align **`PORT`** in `.env` with that URL. |

The Vite app uses **`envDir` = repository root** so `VITE_API_URL`, `DEPLOYWERK_API_PROXY`, and API `.env` stay in one place (not only `web/.env`).

The dev overlay message about **React DevTools** is unrelated to API errors.

### Where to read logs

- **Default (full stack in Docker):** `docker compose logs -f api web` (add `--profile authentik` and Authentik service names if needed).
- **Postgres (Docker):** `docker compose logs -f postgres`
- **API on host:** terminal running `cargo run`

### Database migrations and demo seed

1. **Migrations** run automatically when the API starts (`sqlx` migrate against `DATABASE_URL`).
2. **Demo users and sample project/env/app** run when **`SEED_DEMO_USERS=true`** and **`APP_ENV`** is not `production` (see [`crates/deploywerk-api/src/seed.rs`](crates/deploywerk-api/src/seed.rs)).
3. **Demo passwords on the login page** come from **`GET /api/v1/bootstrap`** when **`DEMO_LOGINS_PUBLIC=true`** (non-production only).

---

## Run with Authentik (OIDC)

Authentik runs in Docker on **9000** (HTTP) and **9443** (HTTPS). DeployWerk’s Postgres stays on **5432**. If `Bind for 0.0.0.0:9000 failed` appears, another process owns that port — stop it or change the left-hand side of `ports:` for `authentik-server` in [docker-compose.yml](docker-compose.yml) (e.g. `9001:9000`).

1. Set strong values in `.env` (see [.env.example](.env.example)):

   - `AUTHENTIK_SECRET_KEY` — long random string  
   - `AUTHENTIK_POSTGRES_PASSWORD` — DB password for Authentik’s Postgres  

2. Start stack:

   ```bash
   docker compose --profile authentik up -d --build
   ```

   Or: `./scripts/deploywerk-dev.sh run --authentik`

3. Wait until Authentik answers (first boot can take 1–2 minutes):

   ```bash
   curl -sf http://127.0.0.1:9000/-/health/live/
   ```

4. **First-time setup:** open http://127.0.0.1:9000/if/admin/ — create the admin account (new volume) and complete the installer.

5. In Authentik: **Applications → Providers** — create an **OAuth2/OpenID** provider. Then **Applications → Applications** — create an application linked to that provider.

6. Copy the **OpenID Configuration Issuer** URL for your application (looks like  
   `http://127.0.0.1:9000/application/o/<slug>/`).

7. In DeployWerk `.env` set:

   | Variable | Example |
   |----------|---------|
   | `AUTHENTIK_ISSUER` | Issuer URL above (no trailing slash inconsistency — API normalizes) |
   | `AUTHENTIK_CLIENT_ID` | Client id from the provider |
   | `AUTHENTIK_CLIENT_SECRET` | Client secret |
   | `AUTHENTIK_REDIRECT_URI` | Must match provider; e.g. `http://127.0.0.1:5173/login/oidc/callback` for local Vite |

   Optional:

   | Variable | Purpose |
   |----------|---------|
   | `AUTHENTIK_BROWSER_BASE_URL` | `http://127.0.0.1:9000` if issuer alone is not enough for admin links |
   | `AUTHENTIK_ADMIN_URL` | Full admin URL override (default derives to `{origin}/if/admin/`) |

8. Restart the API container: `docker compose restart api` (or `./scripts/deploywerk-dev.sh stop` then `run` / `run --authentik`). The **login** page shows **Continue with Authentik** when OIDC is configured, and **Open IdP admin** when an admin URL is resolved.

**Logs (host, not in DeployWerk UI):**

```bash
docker compose --profile authentik logs -f authentik-server authentik-worker
```

**SCIM / Mollie / billing webhooks:** env vars are listed in [.env.example](.env.example); behavior is implemented in `crates/deploywerk-api/src/scim.rs`, `team_platform.rs`, etc.

---

## External deploy worker

If `DEPLOYWERK_DEPLOY_DISPATCH=external`, the API only enqueues jobs. Run:

```bash
cargo run -p deploywerk-api --bin deploywerk-deploy-worker
```

Same `.env` as the API (database + `SERVER_KEY_ENCRYPTION_KEY` + platform/Traefik vars).

---

## Docker Compose logs and troubleshooting

Logs are read on the host where Compose runs; DeployWerk does not stream container logs in the UI.

1. See service status: `docker compose ps` (add `-a` to include stopped containers).
2. Follow recent output from **all** default services:  
   `docker compose logs -f --tail=200`
3. **API and web:** `docker compose logs -f api web`
4. **PostgreSQL** only (DeployWerk DB):  
   `docker compose logs -f postgres`
5. **Authentik** (when using `--profile authentik`):  
   `docker compose --profile authentik logs -f authentik-server authentik-worker`
6. **Port 9000 already in use** (Authentik HTTP): another process is bound to the host port. Either stop that process or change the **left** side of `ports:` for `authentik-server` in [docker-compose.yml](docker-compose.yml) (for example `9001:9000`), then use the new host port in `AUTHENTIK_*` / browser URLs.

If a container exits immediately, `docker compose logs <service>` (without `-f`) usually shows the fatal error.

---

## Configuration highlights

| Variable | Purpose |
|----------|---------|
| `DATABASE_URL` | PostgreSQL (default matches `docker compose` Postgres) |
| `JWT_SECRET` | Session signing |
| `SERVER_KEY_ENCRYPTION_KEY` | 32-byte key for SSH private keys at rest |
| `APP_ENV=production` | Disables demo seeding and demo password exposure |
| `DEPLOYWERK_PLATFORM_DOCKER_ENABLED` | Run `docker` on the API host |
| `DEPLOYWERK_APPS_BASE_DOMAIN` / `DEPLOYWERK_EDGE_MODE` | Traefik-style hostnames |
| `DEPLOYWERK_SMTP_HOST`, `DEPLOYWERK_SMTP_FROM`, … | Transactional SMTP (invites + `email` notification endpoints) — see [.env.example](.env.example) |
| `DEPLOYWERK_PUBLIC_APP_URL` | Public UI origin for invite links in email |

Webhook URLs (prepend your public API origin):

| Path | Notes |
|------|--------|
| `POST /api/v1/hooks/github/{team_id}` | GitHub push; optional `X-Hub-Signature-256` |
| `POST /api/v1/hooks/gitlab/{team_id}` | GitLab push; optional `X-Gitlab-Token` |
| `POST /api/v1/hooks/github-app` | GitHub App; needs `GITHUB_APP_WEBHOOK_SECRET` |

---

## API quick reference

| Method | Path | Auth |
|--------|------|------|
| GET | `/api/v1/health` | No |
| GET | `/api/v1/bootstrap` | No — demo flags, OIDC hints, `idp_admin_url`, mail/public-URL flags |
| POST | `/api/v1/auth/login` | No |
| GET | `/api/v1/me` | Bearer |
| GET | `/api/v1/teams` | Bearer (`read`) |
| POST | `/api/v1/applications/{id}/deploy` | Bearer (`deploy`) |
| POST | `.../applications/{id}/rollback` | Bearer (`deploy`) — prior image must exist |
| GET | `.../applications/{id}/container-log-stream` | Bearer (`read`) — SSE, `docker logs` poll |

Full route list: `rg "route\\(" crates/deploywerk-api/src`

---

## CLI

Install from the workspace (binary name `deploywerk`). Config and JWT are stored under the OS config directory (e.g. `deploywerk-cli/config.json`).

```bash
cargo install --path crates/deploywerk-cli
export DEPLOYWERK_API_URL=http://127.0.0.1:8080   # optional; or pass --base-url each time

deploywerk auth login --email you@example.com     # prompts for password; saves JWT
deploywerk auth status                            # API URL, config path, logged-in yes/no (token never printed)
deploywerk auth logout                            # clears stored JWT

deploywerk teams list
deploywerk teams list --json # machine-readable tables

deploywerk tokens list --json
```

---

## Workspace layout

```
crates/deploywerk-api   # HTTP API + migrations + deploy worker binary
crates/deploywerk-cli
crates/deploywerk-core
crates/deploywerk-agent
web/
docker-compose.yml
```

---

## License

MIT (workspace `Cargo.toml`).
