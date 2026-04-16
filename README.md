# DeployWerk V2

Rust **API** (`deploywerk-api`), **CLI** (`deploywerk-cli`), and **Vite + React** web UI. Teams, projects, environments, Docker applications, deploy jobs (SSH or platform Docker), optional Git webhooks, optional OIDC (e.g. Authentik).

**This file is the only operator documentation in the repository.** Older spec/status markdown was removed; use `git log` / history if you need prior `docs/` content.

**Canonical source:** https://github.com/chavanashutosh/DeployWerk_V2

---

## Where to put the code (Debian 13 production)

**Recommended layout**

| Path | Purpose |
|------|---------|
| `/opt/deploywerk` | Git clone of this repository (builds, `cargo`, `web/`) |
| `/var/lib/deploywerk` | Runtime data (`git-cache`, volumes) for user `deploywerk` |
| `/etc/deploywerk/deploywerk.env` | Secrets and config (`chmod 600`, root or deploywerk-readable only) |
| `/var/www/deploywerk` | Built static SPA (`web/dist` copied here) |

**Dedicated user:** `deploywerk` (system user, home under `/var/lib/deploywerk`). Own code and data: `chown -R deploywerk:deploywerk /opt/deploywerk` (after clone) and `/var/lib/deploywerk`.

**How to upload the tree**

1. **Git (preferred):** On the server, `sudo mkdir -p /opt/deploywerk && sudo chown $USER:$USER /opt/deploywerk`, then clone this repository (HTTPS or SSH):

   ```bash
   cd /opt
   git clone https://github.com/chavanashutosh/DeployWerk_V2.git deploywerk
   # or SSH: git clone git@github.com:chavanashutosh/DeployWerk_V2.git deploywerk
   ```

   Use SSH keys for the SSH URL; never store passwords in the remote URL. For a private fork or self-hosted mirror, use that remote instead.

2. **rsync / scp:** From your workstation, sync the repo to `/opt/deploywerk/` excluding `target/`, `node_modules/`, `.env`.

**Safe baseline**

- PostgreSQL: separate role and database for DeployWerk (not the superuser).
- Firewall: Traefik usually owns **80/443** on the host; only open what you need (SSH, and published Docker ports you intend to expose). Align UFW/nftables with Portainer stacks.
- Secrets: only in `/etc/deploywerk/deploywerk.env` or your secret manager — not committed.

---

## Co-located Docker services (typical single-host stack)

DeployWerk runs **natively** (systemd + API + nginx on a **loopback** port). Other services often run in **Docker** (Portainer-managed): Traefik, Mailcow, Forgejo, Technitium, Matrix (Synapse), etc.

| Service | Typical published ports (host) | Role |
|---------|-------------------------------|------|
| Traefik | 80, 443, 8080 (dashboard) | TLS edge, routes to DeployWerk nginx on loopback |
| Portainer | 9443 | Container UI (optional probe via DeployWerk env) |
| Forgejo | 3000, 2222 (SSH) | Git; webhooks → DeployWerk API |
| Mailcow | 25, 465, 587, 8444, … | SMTP for `DEPLOYWERK_SMTP_*`; UI on 8444 |
| Technitium | 53, 5380 | DNS (optional automation) |
| Synapse | 8008 (internal) | Matrix; `/.well-known` routing in Traefik |

Point **Traefik** at `http://HOST_LOOPBACK:PORT` where nginx serves the SPA (e.g. `127.0.0.1:8085` or Docker bridge IP to host). See [docs/traefik/orbytals-file-provider.example.yml](docs/traefik/orbytals-file-provider.example.yml) for Matrix `/.well-known` priority over the app.

Use `GET /api/v1/bootstrap` and **Team → Integrations** for link slots. Set `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS=true` only when the **API process** can reach integration URLs on `127.0.0.1` (native API on the host). For **Docker Compose**, [.env.example](.env.example) uses `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS=false` and explicit `DEPLOYWERK_INTEGRATION_*_URL` values with **`host.docker.internal`** so the `api` container reaches services published on the host; [docker-compose.yml](docker-compose.yml) adds `extra_hosts: host.docker.internal:host-gateway` on `api` (Linux-friendly).

---

## Quick start (local development)

**Requirements:** Docker Compose v2, repo-root `.env` (from [.env.example](.env.example)), **Rust (`cargo`)** and **Node/npm** on PATH for the default flow. Use Git Bash or WSL on Windows (this script is bash).

**Default — native API + Vite** ([`scripts/deploywerk-dev.sh`](scripts/deploywerk-dev.sh)): Docker runs **Postgres**, **MinIO**, and **minio-init** only. The script starts **deploywerk-api** with `cargo run` and the UI with `npm run dev` in `web/`, with `DATABASE_URL` and MinIO pointed at the published host ports (**5432**, **19000**). Git cache and deploy volumes use repo-local `.deploywerk-git-cache/` and `.deploywerk-volumes/`.

**Full stack in Docker** (optional): same script with **`--docker`** builds and runs **api** + **web** containers plus deps (matches old behavior). Use **`--authentik`** with either mode for OIDC services in Compose.

```bash
chmod +x scripts/deploywerk-dev.sh
./scripts/deploywerk-dev.sh run                    # native API + Vite; Postgres + MinIO in Docker
./scripts/deploywerk-dev.sh run --authentik        # same + Authentik in Docker
./scripts/deploywerk-dev.sh run --docker           # everything in Docker (api + web + deps)
./scripts/deploywerk-dev.sh run --docker --authentik
./scripts/deploywerk-dev.sh stop                   # reads last run mode from .deploywerk-dev.mode
./scripts/deploywerk-dev.sh clean                  # removes containers/volumes; add --rmi-local to prune images
```

| URL / port | Role |
|------------|------|
| http://127.0.0.1:5173 | Vite UI (native default) or nginx front (`--docker`) |
| http://127.0.0.1:8080 | API |
| http://127.0.0.1:19000 / 19001 | MinIO S3 API / console on host |
| 5432 | Postgres on host |

**`.env` for `run --docker`:** use Minio at `http://minio:9000` and **`host.docker.internal`** for host integrations (see [.env.example](.env.example)). **Native default** overrides DB and storage in the script; keep other keys in `.env`.

**Manual host dev** (without the script): `docker compose up -d postgres minio minio-init`, then `cargo run -p deploywerk-api --bin deploywerk-api` and `cd web && npm install && npm run dev` (Vite proxies `/api`; see [web/vite.config.ts](web/vite.config.ts)).

**Windows (PowerShell) without bash:** `docker compose up -d postgres minio minio-init` then start API and Vite as above, or use WSL/Git Bash for the script.

### Web `/api` returns 404

| Cause | Fix |
|--------|-----|
| API not running | `curl -sf http://127.0.0.1:8080/api/v1/health` |
| Static host without proxy | Set `VITE_API_URL` at build time or use nginx/Vite proxy (see [.env.example](.env.example)) |
| API on another port | Set `DEPLOYWERK_API_PROXY` in repo-root `.env` for Vite |

### Logs

- **Native default:** `tail -f .deploywerk-logs/api.log .deploywerk-logs/web.log`
- **Full Docker (`--docker`):** `docker compose logs -f api web` (add `--profile authentik` when used)

### Migrations and demo data

Migrations run when the API starts. Demo users load when `SEED_DEMO_USERS=true` and `APP_ENV` is not `production`. Demo passwords on the login page come from `GET /api/v1/bootstrap` when `DEMO_LOGINS_PUBLIC=true` (non-production only).

---

## Authentik (OIDC) in Docker

In [docker-compose.yml](docker-compose.yml), Authentik is published on **9000** (HTTP) and **9445** (HTTPS, container `9443`) so MinIO can use **19000/19001** on the host and **9443** stays free for tools like Portainer.

1. Set `AUTHENTIK_SECRET_KEY` and `AUTHENTIK_POSTGRES_PASSWORD` in `.env` (see [.env.example](.env.example)).
2. `./scripts/deploywerk-dev.sh run --authentik` (native API + Authentik in Docker) or `docker compose --profile authentik up -d --build` / `./scripts/deploywerk-dev.sh run --docker --authentik` (full stack in Docker)
3. Wait: `curl -sf http://127.0.0.1:9000/-/health/live/`
4. Open **https://127.0.0.1:9445/** or http://127.0.0.1:9000/if/admin/ — complete installer (browser may warn on self-signed TLS).
5. Create OAuth2/OpenID provider + application in Authentik; copy issuer URL.
6. Set `AUTHENTIK_ISSUER`, `AUTHENTIK_CLIENT_ID`, `AUTHENTIK_CLIENT_SECRET`, `AUTHENTIK_REDIRECT_URI` (e.g. `http://127.0.0.1:5173/login/oidc/callback` for local Vite).
7. `docker compose restart api`

Logs: `docker compose --profile authentik logs -f authentik-server authentik-worker`

---

## Production: native API + nginx + systemd (Debian 13)

Typical path on **Debian 13 (trixie)** or compatible: **PostgreSQL** on the host, **deploywerk-api** under **systemd**, **nginx** on a **loopback** port when Traefik fronts TLS, static SPA under `/var/www/deploywerk`.

### Packages and toolchain

```bash
sudo apt update
sudo apt install -y nginx postgresql build-essential pkg-config libssl-dev \
  certbot python3-certbot-nginx git curl
```

Install **Rust** via [rustup](https://rustup.rs/) and **Node.js 22** via [NodeSource](https://github.com/nodesource/distributions) or `nvm` — match versions used for production builds.

### Docker (optional on the API host)

Only required if you enable **Platform Docker** deploys (`DEPLOYWERK_PLATFORM_DOCKER_ENABLED=true`). The `docker` group is effectively root — use deliberately.

### Service user and directories

```bash
sudo useradd --system --create-home --home-dir /var/lib/deploywerk --shell /usr/sbin/nologin deploywerk
sudo mkdir -p /opt/deploywerk /var/lib/deploywerk/git-cache /var/lib/deploywerk/volumes /etc/deploywerk
sudo chown -R deploywerk:deploywerk /var/lib/deploywerk
```

Place your clone at `/opt/deploywerk` and `chown -R deploywerk:deploywerk /opt/deploywerk` after checkout.

### Database

Create PostgreSQL user and database `deploywerk` (Debian: `sudo -u postgres createuser`, `createdb`, `psql` …).

### Environment file

Create `/etc/deploywerk/deploywerk.env` (`chmod 600`). Minimal:

```bash
APP_ENV=production
DATABASE_URL=postgresql://deploywerk:PASSWORD@127.0.0.1:5432/deploywerk
JWT_SECRET=...                    # openssl rand -base64 48
SERVER_KEY_ENCRYPTION_KEY=...     # openssl rand -hex 32
HOST=127.0.0.1
PORT=8080
DEPLOYWERK_PUBLIC_APP_URL=https://app.example.com
```

See [.env.example](.env.example) for SMTP (Mailcow submission), Platform Docker, Traefik edge, integration URLs, OIDC, SCIM, etc.

### Build and install

```bash
cd /opt/deploywerk
sudo -u deploywerk git pull   # or your deploy procedure
sudo -u deploywerk cargo build --release -p deploywerk-api --bin deploywerk-api
sudo -u deploywerk cargo build --release -p deploywerk-api --bin deploywerk-deploy-worker
sudo install -m 0755 target/release/deploywerk-api /usr/local/bin/deploywerk-api
sudo install -m 0755 target/release/deploywerk-deploy-worker /usr/local/bin/deploywerk-deploy-worker
cd web && npm ci && npm run build
sudo mkdir -p /var/www/deploywerk && sudo cp -a dist/* /var/www/deploywerk/
sudo chown -R deploywerk:deploywerk /opt/deploywerk /var/lib/deploywerk
```

### systemd

**deploywerk-api.service** — `User=deploywerk`, `WorkingDirectory=/opt/deploywerk`, `EnvironmentFile=/etc/deploywerk/deploywerk.env`, `ExecStart=/usr/local/bin/deploywerk-api`.

If `DEPLOYWERK_DEPLOY_DISPATCH=external`, run **deploywerk-deploy-worker** in a separate unit with the same env file.

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now deploywerk-api
journalctl -u deploywerk-api -f
```

### nginx

Example: `root /var/www/deploywerk`; `location /api/` → `proxy_pass http://127.0.0.1:8080` with `X-Forwarded-*`; `location /` → `try_files` for SPA.

- If **nginx terminates TLS** on this host: use **certbot** (`sudo certbot --nginx -d app.example.com`) after DNS points to the server.
- If **Traefik terminates TLS**, bind nginx only to `127.0.0.1:8085` (or similar); do not compete for public 80/443.

### Verify

```bash
curl -sf http://127.0.0.1:8080/api/v1/health
curl -sf http://127.0.0.1:8080/api/v1/bootstrap | head
```

Browser: public URL — SPA loads; `/api/v1/bootstrap` not 404.

### External deploy worker

If `DEPLOYWERK_DEPLOY_DISPATCH=external`, run `deploywerk-deploy-worker` with the same `DATABASE_URL` and keys as the API.

---

## Traefik on the same host (DeployWerk native, edge in Docker)

When **Traefik** already owns **80/443**, DeployWerk’s nginx must **not** bind the same public ports for the same hostname.

1. **API** listens on `127.0.0.1:8080`.
2. **nginx** serves SPA + `/api` proxy on a **loopback port** (e.g. `127.0.0.1:8085`).
3. **Traefik** routes `Host(your.domain)` to the host (e.g. `http://172.17.0.1:8085` via Docker bridge gateway, or `host.docker.internal` where supported).

### Matrix + apex

If Synapse uses the same apex domain, `/.well-known/matrix/*` must reach Synapse. Use **higher Traefik priority** for `PathPrefix(\`/.well-known/matrix\`)` than the catch-all DeployWerk router. Example: [docs/traefik/orbytals-file-provider.example.yml](docs/traefik/orbytals-file-provider.example.yml).

TLS is usually **Traefik ACME**, not certbot on the loopback nginx.

### Example env (public site)

- `DEPLOYWERK_PUBLIC_APP_URL=https://your.domain`
- `HOST=127.0.0.1`, `PORT=8080`
- SMTP via Mailcow: `DEPLOYWERK_SMTP_*`
- Platform Docker + Traefik labels: `DEPLOYWERK_PLATFORM_DOCKER_ENABLED`, `DEPLOYWERK_EDGE_MODE=traefik`, `DEPLOYWERK_TRAEFIK_DOCKER_NETWORK`, `DEPLOYWERK_APPS_BASE_DOMAIN`

### Orbytals edge example (Ubuntu)

Example Traefik layout (`traefik/` + `traefik-labels/`) for **orbytals.com** / **hermesapp.live** lives under [examples/orbytals-traefik-edge](examples/orbytals-traefik-edge). On the server, install it under a single root such as **`/opt/orbytals/edge`** (the script default). Fresh host: run [scripts/server-bootstrap-orbytals.sh](scripts/server-bootstrap-orbytals.sh) first (Docker, external **`proxy`** network, `/opt/traefik/acme/acme.json`), then from a clone of this repo:

```bash
sudo bash scripts/traefik-edge-migrate-orbytals.sh all
```

Override paths with `EDGE_ROOT`, `SOURCE_TREE`, and optional `MAILCOW_DIR`, `TECHNITIUM_COMPOSE_DIR`, `FORGEJO_APP_INI` / `FORGEJO_DATA_DIR`, `SYNAPSE_HOMESERVER_YAML` / `SYNAPSE_DATA_DIR` — see the header comment in [scripts/traefik-edge-migrate-orbytals.sh](scripts/traefik-edge-migrate-orbytals.sh). Match DeployWerk to the same Docker network name as Traefik (example stack uses **`proxy`**: set `DEPLOYWERK_TRAEFIK_DOCKER_NETWORK=proxy`).

---

## Environment variables (summary)

Authoritative list: [.env.example](.env.example) and `crates/deploywerk-api/src/config.rs`.

| Area | Examples |
|------|-----------|
| Core | `DATABASE_URL`, `JWT_SECRET`, `SERVER_KEY_ENCRYPTION_KEY`, `APP_ENV`, `HOST`, `PORT` |
| Public UI | `DEPLOYWERK_PUBLIC_APP_URL` (invite links) |
| Mail | `DEPLOYWERK_SMTP_*` (Mailcow submission host/port/user) |
| OIDC | `AUTHENTIK_ISSUER`, `AUTHENTIK_CLIENT_ID`, `AUTHENTIK_CLIENT_SECRET`, `AUTHENTIK_REDIRECT_URI` |
| Platform Docker | `DEPLOYWERK_PLATFORM_DOCKER_ENABLED`, `DEPLOYWERK_APPS_BASE_DOMAIN`, `DEPLOYWERK_EDGE_MODE`, `DEPLOYWERK_TRAEFIK_DOCKER_NETWORK` |
| Integration links (UI + bootstrap) | `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS=true` or `DEPLOYWERK_INTEGRATION_*_URL` (Forgejo, Mailcow, Portainer, Technitium, Matrix client, Traefik dashboard) |
| Docs link | `DEPLOYWERK_DOCUMENTATION_BASE_URL` — SSO help links to `{base}/README.md#single-sign-on-oidc` |
| Optional probes | `DEPLOYWERK_PORTAINER_INTEGRATION_*`, `DEPLOYWERK_TECHNITIUM_DNS_*` |

**Note:** If the API runs **inside** Docker, `127.0.0.1` is the container, not the host — use explicit URLs instead of `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS` for host-published ports.

---

## Single sign-on (OIDC)

DeployWerk uses **Authentik-shaped** env vars (`AUTHENTIK_*`). In your IdP, create an OAuth2/OpenID app; **redirect URI** must match `AUTHENTIK_REDIRECT_URI`. In-app help: `/app/sso-setup`.

**Redirect URI examples**

| App | Path (typical) |
|-----|----------------|
| DeployWerk | `https://<app>/login/oidc/callback` |
| Forgejo / Portainer | Per their OAuth settings |

Optional: `DEPLOYWERK_SCIM_BEARER_TOKEN` for SCIM provisioning (see `.env.example`).

Forgejo **deploy automation** uses GitLab-style webhooks: `POST /api/v1/hooks/gitlab/{team_id}` (separate from SSO).

---

## Mail

### Production / Mailcow

Point `DEPLOYWERK_SMTP_*` at your SMTP submission host (e.g. `mail.example.com:587` STARTTLS). Create a mailbox or SMTP credentials in Mailcow for DeployWerk.

Team mail product features (when enabled in code) are documented in this README only; there is no separate spec file in-repo.

### Local dev (Compose)

The default [docker-compose.yml](docker-compose.yml) does **not** bundle a mail server or webmail (avoids host port clashes such as `8082`). [.env.example](.env.example) sets `DEPLOYWERK_SMTP_HOST=host.docker.internal` and port `587` so the **api** container can reach **Mailcow** on the host; the same file uses `host.docker.internal` for integration links. Compose adds `extra_hosts: host.docker.internal:host-gateway` on the `api` service. If the API runs **on the host** (not in Compose), use `127.0.0.1` for SMTP and integrations instead. Self-signed HTTPS to Portainer/Mailcow from inside the container may require extra TLS trust configuration.

---

## Production checklist

- [ ] DNS **A/AAAA** to server; TLS (Let’s Encrypt or Traefik ACME) and renewal tested.
- [ ] Firewall: **22**, **80**, **443** (and mail/DNS ports only if those services run on the same host).
- [ ] `APP_ENV=production`; strong `JWT_SECRET` and `SERVER_KEY_ENCRYPTION_KEY`; `/etc/deploywerk/deploywerk.env` **0600** and backed up.
- [ ] PostgreSQL up; backups + tested restore path.
- [ ] `DEPLOYWERK_PUBLIC_APP_URL` and optional `DEPLOYWERK_SMTP_*` for email.
- [ ] Inline vs `DEPLOYWERK_DEPLOY_DISPATCH=external` worker decided; worker systemd enabled if external.
- [ ] `curl -sf http://127.0.0.1:8080/api/v1/health` and browser SPA + `/api/v1/bootstrap` OK.

---

## Webhooks (reference)

| Method | Path |
|--------|------|
| POST | `/api/v1/hooks/github/{team_id}` |
| POST | `/api/v1/hooks/gitlab/{team_id}` |
| POST | `/api/v1/hooks/github-app` (GitHub App) |

Prepend your public API origin. Secrets: see [.env.example](.env.example).

---

## Firewall bootstrap script

[scripts/server-bootstrap-orbytals.sh](scripts/server-bootstrap-orbytals.sh) installs base packages, Docker, UFW holes for mail/Matrix/DNS — use as a **starting point**, then finish Traefik/Mailcow/Matrix/DNS per your stack.

---

## Optional: remote desktop / Hestia

XRDP and HestiaCP are **optional** and conflict with DeployWerk if they fight for the same **80/443** on one machine — keep separate hosts or one reverse proxy owner.

---

## Tight fit with your Docker stack (roadmap)

Use these in order; all exist or are stubbed in the current codebase:

- **`GET /api/v1/bootstrap`** — non-secret integration URLs for Traefik, Forgejo, Mailcow, Portainer, Technitium, Matrix client, etc.
- **`DEPLOYWERK_LOCAL_SERVICE_DEFAULTS`** — one-shot fill for typical single-host `127.0.0.1` ports when the API runs on the **host** (not inside Docker). With Compose, prefer explicit `DEPLOYWERK_INTEGRATION_*_URL` using `host.docker.internal` (see [.env.example](.env.example)).
- **Forgejo / GitHub / GitLab** — configure webhooks to DeployWerk team endpoints (see Webhooks table).
- **OIDC** — align `AUTHENTIK_*` with your IdP; optional SCIM.
- **Platform Docker + Traefik** — `DEPLOYWERK_EDGE_MODE=traefik` and app container labels when deploying user apps on the same Traefik network.
- **Portainer / Technitium** — optional read-only probes via env (platform admin).

---

## API quick reference

| Method | Path | Auth |
|--------|------|------|
| GET | `/api/v1/health` | No |
| GET | `/api/v1/bootstrap` | No |
| POST | `/api/v1/auth/login` | No |
| GET | `/api/v1/me` | Bearer |

Full routes: `rg "route\\(" crates/deploywerk-api/src`

---

## CLI

```bash
cargo install --path crates/deploywerk-cli
export DEPLOYWERK_API_URL=http://127.0.0.1:8080
deploywerk auth login --email you@example.com
deploywerk teams list
```

---

## Repository layout

```
crates/deploywerk-api   # HTTP API + migrations + deploy worker binary
crates/deploywerk-cli
crates/deploywerk-core
crates/deploywerk-agent
web/
docker-compose.yml
docs/traefik/           # example Traefik routes (YAML)
```

## License

MIT (workspace `Cargo.toml`).
