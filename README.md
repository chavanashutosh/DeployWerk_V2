# DeployWerk V2

Rust **API** (`deploywerk-api`), **CLI** (`deploywerk-cli`), and **Vite + React** web UI. Teams, projects, environments, Docker applications, deploy jobs (SSH or platform Docker), optional Git webhooks, optional OIDC (e.g. Authentik).

**This file is the only operator documentation in the repository.** Older spec/status markdown was removed; use `git log` / history if you need prior `docs/` content.

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

1. **Git (preferred):** On the server, `sudo mkdir -p /opt/deploywerk && sudo chown $USER:$USER /opt/deploywerk`, then clone your private remote (e.g. Forgejo on port `3000`, Git over SSH on host port `2222`):

   ```bash
   cd /opt
   git clone git@git.example.com:you/deploywerk.git deploywerk
   ```

   Use SSH keys; never store passwords in the remote URL.

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

Use `GET /api/v1/bootstrap` and **Team → Integrations** for link slots. Set `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS=true` only when the **API process** can reach those URLs on `127.0.0.1` (native API on host); if the API ran in Docker, use explicit `DEPLOYWERK_INTEGRATION_*_URL` values instead.

---

## Quick start (Docker Compose)

**Requirements:** Docker Compose v2. Copy [.env.example](.env.example) to `.env` at the repo root.

```bash
chmod +x scripts/deploywerk-dev.sh
./scripts/deploywerk-dev.sh run              # Postgres + API + web (nginx on :5173)
./scripts/deploywerk-dev.sh run --authentik  # + Authentik (OIDC)
./scripts/deploywerk-dev.sh stop
./scripts/deploywerk-dev.sh clean            # removes DB volumes (use --authentik if needed)
```

- **Web UI:** http://127.0.0.1:5173  
- **API:** http://127.0.0.1:8080  

**Windows (PowerShell):** `docker compose up -d --build` (add `--profile authentik` for Authentik).

Compose sets `DATABASE_URL` for the `api` service to the `postgres` container; your `.env` may still say `127.0.0.1` for host tools.

### Host development (API + Vite on the machine, Postgres in Docker)

1. `docker compose up -d postgres`
2. `cargo run -p deploywerk-api --bin deploywerk-api`
3. `cd web && npm install && npm run dev` → http://127.0.0.1:5173 (Vite proxies `/api`; see [web/vite.config.ts](web/vite.config.ts))

### Web `/api` returns 404

| Cause | Fix |
|--------|-----|
| API not running | `curl -sf http://127.0.0.1:8080/api/v1/health` |
| Static host without proxy | Set `VITE_API_URL` at build time or use nginx/Vite proxy (see [.env.example](.env.example)) |
| API on another port | Set `DEPLOYWERK_API_PROXY` in repo-root `.env` for Vite |

### Logs (Compose)

`docker compose logs -f api web` — add `--profile authentik` and service names if used.

### Migrations and demo data

Migrations run when the API starts. Demo users load when `SEED_DEMO_USERS=true` and `APP_ENV` is not `production`. Demo passwords on the login page come from `GET /api/v1/bootstrap` when `DEMO_LOGINS_PUBLIC=true` (non-production only).

---

## Authentik (OIDC) in Docker

Authentik uses host ports **9000** / **9443** by default. If port 9000 is busy, change the **left** side of `ports:` for `authentik-server` in [docker-compose.yml](docker-compose.yml).

1. Set `AUTHENTIK_SECRET_KEY` and `AUTHENTIK_POSTGRES_PASSWORD` in `.env` (see [.env.example](.env.example)).
2. `docker compose --profile authentik up -d --build` or `./scripts/deploywerk-dev.sh run --authentik`
3. Wait: `curl -sf http://127.0.0.1:9000/-/health/live/`
4. Open http://127.0.0.1:9000/if/admin/ — complete installer.
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

### Local dev (Compose): Stalwart + Isotope

The default [docker-compose.yml](docker-compose.yml) can include Stalwart and webmail proxied at `/mail/` on the dev nginx. Example `.env`:

```env
DEPLOYWERK_SMTP_HOST=stalwart
DEPLOYWERK_SMTP_PORT=587
DEPLOYWERK_SMTP_TLS=starttls
DEPLOYWERK_SMTP_FROM=DeployWerk <noreply@dev.local>
DEPLOYWERK_SMTP_USER=deploywerk
DEPLOYWERK_SMTP_PASSWORD=deploywerk-dev-only-change-me
```

Stalwart admin is typically on host **8082** in that layout; configure domain and users in Stalwart’s UI, then `docker compose restart api`.

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
- **`DEPLOYWERK_LOCAL_SERVICE_DEFAULTS`** — one-shot fill for typical single-host `127.0.0.1` ports when the API runs on the **host** (not inside Docker).
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
