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

Point **Traefik** at `http://HOST_LOOPBACK:PORT` where nginx serves the SPA (e.g. `localhost:8085` or Docker bridge IP to host). See [docs/traefik/orbytals-file-provider.example.yml](docs/traefik/orbytals-file-provider.example.yml) for Matrix `/.well-known` priority over the app.

Use `GET /api/v1/bootstrap` and **Team → Integrations** for link slots. Set `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS=true` only when the **API process** can reach integration URLs on `localhost` (native API on the host); when unset, `APP_ENV=development` enables the same preset merge. For **Docker Compose**, set `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS=false` and use explicit `DEPLOYWERK_INTEGRATION_*_URL` values with **`host.docker.internal`** (as in [.env.example](.env.example)) so the `api` container reaches services published on the host; [docker-compose.yml](docker-compose.yml) adds `extra_hosts: host.docker.internal:host-gateway` on `api` (Linux-friendly).

---

## Quick start (local development)

**Requirements:** Docker Compose v2, repo-root `.env` (from [.env.example](.env.example)), **Rust (`cargo`)** and **Node/npm** on PATH.

**Default local dev:** run **Postgres**, **MinIO**, and optional **Authentik** in Docker; run **deploywerk-api** and Vite on the host. Git cache and deploy volumes use repo-local `.deploywerk-git-cache/` and `.deploywerk-volumes/`.

```bash
docker compose up -d postgres minio minio-init
cargo run -p deploywerk-api --bin deploywerk-api
cd web && npm install && npm run dev
```

| URL / port | Role |
|------------|------|
| http://localhost:5173 | Vite UI (native default) or nginx front (`--docker`) |
| http://localhost:8080 | API |
| http://localhost:19000 / 19001 | MinIO S3 API / console on host |
| 5432 | Postgres on host |

**Windows (PowerShell) without bash:** `docker compose up -d postgres minio minio-init` then start API and Vite as above.

### Web `/api` returns 404

| Cause | Fix |
|--------|-----|
| API not running | `curl -sf http://localhost:8080/api/v1/health` |
| Static host without proxy | Set `VITE_API_URL` at build time or use nginx/Vite proxy (see [.env.example](.env.example)) |
| API on another port | Set `DEPLOYWERK_API_PROXY` in repo-root `.env` for Vite |

### Logs

- Host API: terminal output from `cargo run`
- Vite UI: terminal output from `npm run dev`
- Docker services: `docker compose logs -f postgres minio minio-init`

### Migrations and demo data

Migrations run when the API starts. Demo users load when `SEED_DEMO_USERS=true` and `APP_ENV` is not `production`. Demo passwords on the login page come from `GET /api/v1/bootstrap` when `DEMO_LOGINS_PUBLIC=true` (non-production only).

---

## Authentik (OIDC) in Docker

In [docker-compose.yml](docker-compose.yml), Authentik is published on **9000** (HTTP) and **9445** (HTTPS, container `9443`) so MinIO can use **19000/19001** on the host and **9443** stays free for tools like Portainer.

1. Set `AUTHENTIK_SECRET_KEY` and `AUTHENTIK_POSTGRES_PASSWORD` in `.env` (see [.env.example](.env.example)).
2. `docker compose --profile authentik up -d`
3. Wait: `curl -sf http://localhost:9000/-/health/live/`
4. Open **https://localhost:9445/** or http://localhost:9000/if/admin/ — complete installer (browser may warn on self-signed TLS).
5. Create OAuth2/OpenID provider + application in Authentik; copy issuer URL.
6. Set `AUTHENTIK_ISSUER`, `AUTHENTIK_CLIENT_ID`, `AUTHENTIK_CLIENT_SECRET`, `AUTHENTIK_REDIRECT_URI` (e.g. `http://localhost:5173/login/oidc/callback` for local Vite).
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
DATABASE_URL=postgresql://deploywerk:PASSWORD@localhost:5432/deploywerk
JWT_SECRET=...                    # openssl rand -base64 48
SERVER_KEY_ENCRYPTION_KEY=...     # openssl rand -hex 32
HOST=localhost
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

Example: `root /var/www/deploywerk`; `location /api/` → `proxy_pass http://localhost:8080` with `X-Forwarded-*`; `location /` → `try_files` for SPA.

- If **nginx terminates TLS** on this host: use **certbot** (`sudo certbot --nginx -d app.example.com`) after DNS points to the server.
- If **Traefik terminates TLS**, bind nginx only to `localhost:8085` (or similar); do not compete for public 80/443.

### Verify

```bash
curl -sf http://localhost:8080/api/v1/health
curl -sf http://localhost:8080/api/v1/bootstrap | head
```

Browser: public URL — SPA loads; `/api/v1/bootstrap` not 404.

### External deploy worker

If `DEPLOYWERK_DEPLOY_DISPATCH=external`, run `deploywerk-deploy-worker` with the same `DATABASE_URL` and keys as the API.

---

## Traefik on the same host (DeployWerk native, edge in Docker)

When **Traefik** already owns **80/443**, DeployWerk’s nginx must **not** bind the same public ports for the same hostname.

1. **API** listens on `localhost:8080`.
2. **nginx** serves SPA + `/api` proxy on a **loopback port** (e.g. `localhost:8085`).
3. **Traefik** routes `Host(your.domain)` to the host (e.g. `http://172.17.0.1:8085` via Docker bridge gateway, or `host.docker.internal` where supported).

### Matrix + apex

If Synapse uses the same apex domain, `/.well-known/matrix/*` must reach Synapse. Use **higher Traefik priority** for `PathPrefix(\`/.well-known/matrix\`)` than the catch-all DeployWerk router. Example: [docs/traefik/orbytals-file-provider.example.yml](docs/traefik/orbytals-file-provider.example.yml).

TLS is usually **Traefik ACME**, not certbot on the loopback nginx.

### Example env (public site)

- `DEPLOYWERK_PUBLIC_APP_URL=https://your.domain`
- `HOST=localhost`, `PORT=8080`
- SMTP via Mailcow: `DEPLOYWERK_SMTP_*`
- Platform Docker + Traefik labels: `DEPLOYWERK_PLATFORM_DOCKER_ENABLED`, `DEPLOYWERK_EDGE_MODE=traefik`, `DEPLOYWERK_TRAEFIK_DOCKER_NETWORK`, `DEPLOYWERK_APPS_BASE_DOMAIN`

### Orbytals one-script installer (Ubuntu)

The supported production installer is now [scripts/orbytals-install.sh](scripts/orbytals-install.sh). It provisions the Ubuntu 24 host and wires the Orbytals stack together behind Traefik:

- Traefik
- DeployWerk (native `systemd` + loopback `nginx`)
- Mailcow / webmail
- Forgejo
- Synapse / Matrix
- Technitium DNS (published on host **8053/tcp+udp**; not standard **53**)
- Cockpit
- Garage object storage bootstrap

Run it from a clone of this repo:

```bash
sudo bash scripts/orbytals-install.sh all
```

The script prompts interactively for operator credentials and stores its managed runtime state under `/etc/orbytals`. Re-runs are intended to be idempotent.

**Loopback host:** **`127.0.0.1`** and **`localhost`** are both valid for DeployWerk API/nginx, `DATABASE_URL`, SMTP, etc. Defaults use **`localhost`**. **Docker Compose `ports:`** host addresses must be numeric IPs — the installer maps loopback names to **`127.0.0.1`** for Traefik, Garage, Technitium, and Mailcow publish binds. Traefik **`curl --resolve`** checks default to **`CURL_TRAEFIK_LOOPBACK_IP=127.0.0.1`**. See **`sudo bash scripts/orbytals-install.sh --help`** for the full env list.

**TLS / Let's Encrypt:** The installer does **not** run `certbot`. **Traefik** terminates HTTPS on **443** and obtains certificates from **Let's Encrypt** via the **ACME HTTP-01** challenge on **80** (see `certificatesResolvers.le` in the bundled Traefik static config). Certificates are stored in `/opt/traefik/acme/acme.json`. During install you supply **`ACME_EMAIL`** for Let's Encrypt registration. Plain HTTP on **80** redirects to HTTPS except `/.well-known/acme-challenge`, which ACME needs. **Mailcow** is set to **`SKIP_LETS_ENCRYPT=y`** so it does not request its own certs; Traefik still serves **`https://mail.<domain>`** with LE. Until DNS points at this host and ACME finishes, browsers may warn about the certificate; the script's verify step may use `curl -k` while polling.

**Mailcow Docker network:** If **`docker compose up`** fails with **pool overlaps** on **`mailcow-network`**, the installer picks a free **`IPV4_NETWORK`** (Mailcow internal **`/24`**) using **`python3`** + **`docker network inspect`**, sets **`ENABLE_IPV6=false`** by default (many overlaps are IPv6; set **`MAILCOW_ENABLE_IPV6=true`** if you need it), removes a stale **`${COMPOSE_PROJECT_NAME}_mailcow-network`** (default **`mailcowdockerized_mailcow-network`**) before **`up`**, and falls back to **`10.254.99`** if no candidate fits. Override with **`MAILCOW_IPV4_NETWORK`** / **`MAILCOW_IPV6_NETWORK`**. See [docs/orbytals-install-verification.md](docs/orbytals-install-verification.md).

Useful follow-up commands:

```bash
sudo bash scripts/orbytals-install.sh verify
sudo bash scripts/orbytals-install.sh clean
sudo bash scripts/orbytals-install.sh redeploy
```

Managed install roots default to:

- `/opt/orbytals/edge`
- `/opt/orbytals/services`
- `/opt/mailcow-dockerized`
- `/etc/deploywerk/deploywerk.env`

The installer assumes public DNS already points the following names at the Traefik host:

- `orbytals.com` (apex; same DeployWerk site as `app.`, required for HTTPS on the apex host)
- `app.orbytals.com`
- `api.orbytals.com`
- `mail.orbytals.com`
- `git.orbytals.com`
- `dns.orbytals.com`
- `traefik.orbytals.com`
- `cockpit.orbytals.com`
- `chat.hermesapp.live`
- `api.hermesapp.live`

### URLs

External public URLs (via Traefik):

- `https://orbytals.com` (apex; same SPA as `app.`)
- `https://app.orbytals.com`
- `https://api.orbytals.com` (API routes like `/api/v1/health`, `/api/v1/bootstrap`)
- `https://mail.orbytals.com`
- `https://git.orbytals.com`
- `https://dns.orbytals.com` (Technitium web UI)
- `https://traefik.orbytals.com`
- `https://cockpit.orbytals.com`
- `https://chat.hermesapp.live` (Matrix/Synapse, e.g. `/_matrix/client/versions`)
- `https://api.hermesapp.live` (Synapse homeserver API, e.g. `/_matrix/client/versions`)

Local-only URLs (host loopback):

- `http://localhost:8080` (DeployWerk API)
- `http://localhost:8085` (DeployWerk nginx front)
- `http://localhost:18080` (Traefik dashboard local bind)
- `http://localhost:3900` (Garage S3)
- `http://localhost:5380` (Technitium web UI loopback bind)
- `http://localhost:8082` and `https://localhost:8444` (Mailcow loopback binds)

Internal Docker-network URLs (container-to-container):

- `http://synapse:8008` (Traefik routes `chat.hermesapp.live` to this on the `proxy` network)

### Matrix / mobile app connection

- **Web chat**: `https://chat.hermesapp.live` (Element Web)
- **Mobile app homeserver URL**: **`https://api.hermesapp.live`**
  - `https://chat.hermesapp.live` also serves `/.well-known/matrix/client` and `/.well-known/matrix/server` for clients that support discovery.

### Ports

Public inbound ports opened by the installer by default:

| Port | Protocol | Service |
|------|----------|---------|
| `22` | TCP | SSH |
| `80` | TCP | Traefik HTTP / ACME |
| `443` | TCP | Traefik HTTPS |
| `25` | TCP | Mailcow SMTP |
| `465` | TCP | Mailcow SMTPS |
| `587` | TCP | Mailcow submission |
| `110` | TCP | Mailcow POP3 |
| `995` | TCP | Mailcow POP3S |
| `143` | TCP | Mailcow IMAP |
| `993` | TCP | Mailcow IMAPS |
| `4190` | TCP | Mailcow ManageSieve |
| `8053` | TCP/UDP | Technitium DNS (non-standard; avoids host stub resolvers on `53`) |
| `8448` | TCP | Matrix federation |

Loopback-only host ports used by the installer:

| Port | Protocol | Service |
|------|----------|---------|
| `8080` | TCP | DeployWerk API |
| `8085` | TCP | DeployWerk nginx |
| `8082` | TCP | Mailcow HTTP binding for Traefik |
| `8444` | TCP | Mailcow HTTPS binding for host-local use |
| `9292` | TCP | Cockpit host socket, proxied by Traefik by default |
| `18080` | TCP | Traefik local dashboard bind |
| `3900` | TCP | Garage S3 API |
| `3902` | TCP | Garage web endpoint |
| `3903` | TCP | Garage admin API |
| `5380` | TCP | Technitium web UI, proxied by Traefik |

Container-exposed or service-specific ports:

| Port | Protocol | Service |
|------|----------|---------|
| `2222` | TCP | Forgejo SSH |
| `3000` | TCP | Forgejo HTTP inside Docker network |
| `8008` | TCP | Synapse HTTP inside Docker network |

The installer keeps Cockpit direct `9292` access blocked by UFW unless `OPEN_COCKPIT_PORT=true` is explicitly set. Traefik’s local dashboard bind uses `localhost:18080` so it does not collide with the native DeployWerk API on `localhost:8080`.

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

**Note:** If the API runs **inside** Docker, `localhost` is the container, not the host — use explicit URLs instead of `DEPLOYWERK_LOCAL_SERVICE_DEFAULTS` for host-published ports.

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

The default [docker-compose.yml](docker-compose.yml) does **not** bundle a mail server or webmail (avoids host port clashes such as `8082`). [.env.example](.env.example) sets `DEPLOYWERK_SMTP_HOST=host.docker.internal` and port `587` so the **api** container can reach **Mailcow** on the host; the same file uses `host.docker.internal` for integration links. Compose adds `extra_hosts: host.docker.internal:host-gateway` on the `api` service. If the API runs **on the host** (not in Compose), use `localhost` for SMTP and integrations instead. Self-signed HTTPS to Portainer/Mailcow from inside the container may require extra TLS trust configuration.

---

## Production checklist

- [ ] DNS **A/AAAA** to server; TLS (Let’s Encrypt or Traefik ACME) and renewal tested.
- [ ] Firewall: **22**, **80**, **443** (and mail/DNS ports only if those services run on the same host).
- [ ] `APP_ENV=production`; strong `JWT_SECRET` and `SERVER_KEY_ENCRYPTION_KEY`; `/etc/deploywerk/deploywerk.env` **0600** and backed up.
- [ ] PostgreSQL up; backups + tested restore path.
- [ ] `DEPLOYWERK_PUBLIC_APP_URL` and optional `DEPLOYWERK_SMTP_*` for email.
- [ ] Inline vs `DEPLOYWERK_DEPLOY_DISPATCH=external` worker decided; worker systemd enabled if external.
- [ ] `curl -sf http://localhost:8080/api/v1/health` and browser SPA + `/api/v1/bootstrap` OK.

---

## Webhooks (reference)

| Method | Path |
|--------|------|
| POST | `/api/v1/hooks/github/{team_id}` |
| POST | `/api/v1/hooks/gitlab/{team_id}` |
| POST | `/api/v1/hooks/github-app` (GitHub App) |

Prepend your public API origin. Secrets: see [.env.example](.env.example).

---

## Installer notes

[scripts/orbytals-install.sh](scripts/orbytals-install.sh) performs host bootstrap itself: packages, Docker, Cockpit, Node.js 22, Rust, nginx/PostgreSQL prerequisites, Traefik, Garage, and the managed application/service stacks.

Interpret **`verify`** output (timeouts vs `404`, Traefik vs loopback): [docs/orbytals-install-verification.md](docs/orbytals-install-verification.md).

Cockpit-specific behavior in the installer:

- installs `cockpit-storaged`, `udisks2-lvm2`, and `udisks2-btrfs` for storage support, with `udisks2-iscsi` used when available in apt sources
- installs `cockpit-packagekit` and `packagekit` for software updates
- enables `NetworkManager` by default for Cockpit update support on Ubuntu server
- exposes Cockpit primarily through `https://cockpit.orbytals.com`

---

## Troubleshooting (installer)

### `/etc/orbytals/install.env: line N: …: command not found`

That usually means a line in the state file was not a valid `KEY=value` assignment (often a **secret split across lines**, e.g. part of a Garage key on its own row), or there are **spaces after `=`** (bash then treats the next token as a command, e.g. `GARAGE_ACCESS_KEY_ID=             GK7c…`). Current installers **sanitize** `/etc/orbytals/install.env` before sourcing it (drop invalid lines, trim values, re-quote assignments), so rerunning `scripts/orbytals-install.sh` should clear the error. If a secret was truncated, remove the affected key from the file or run `sudo rm -f /etc/orbytals/install.env` and run the installer again so prompts regenerate state.

### Post-install `verify` shows HTTPS failures from the server itself

`curl https://app.example.com` from **the same machine** often fails when DNS points at the server’s public IP but the router does **not** support hairpin NAT (TCP never reaches Traefik). The installer’s smoke checks therefore use **`curl --resolve …:443:127.0.0.1`** (numeric loopback; `curl --resolve` expects an IP) so Traefik is exercised on loopback with the correct hostname. Client machines still need correct **public DNS** (A/AAAA) to reach the site from the Internet.

Timeouts to **app** / **api** / **cockpit** while loopback `8085` / `8080` succeed usually mean Traefik (Docker) could not reach **host** nginx or Cockpit (listen address + UFW); see [docs/orbytals-install-verification.md](docs/orbytals-install-verification.md).

### Garage bootstrap fails with Docker `409` / `unable to upgrade to tcp, received 409`

This usually means the `garage` container is **not running** or is **restarting** when the installer tries to `docker exec` into it.

Check:

```bash
docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'
docker logs garage --tail 200
```

### Garage: `Invalid RPC secret key: expected 32 bytes of random hex`

Garage’s `rpc_secret` in `garage.toml` must be **exactly 64 hexadecimal characters** (32 random bytes). The installer generates this automatically. If you see this error from an older run, pull an updated `scripts/orbytals-install.sh` and re-run the Garage step (or delete the bad `GARAGE_RPC_SECRET` from the installer state file so it can be regenerated).

### Garage restarts with `missing field root_domain` under `[s3_web]`

Recent Garage releases require `root_domain` in the `[s3_web]` section of `garage.toml`. The installer writes a default (`.web.garage.localhost`); set `GARAGE_S3_WEB_ROOT_DOMAIN` before install if you use a real DNS suffix for bucket websites. If you upgraded from an older generated config, add `root_domain` next to `bind_addr` under `[s3_web]` and run `docker compose up -d` again in the Garage directory (often `/opt/orbytals/garage` or `GARAGE_DIR`).

### Garage vs other object storage

DeployWerk only needs an **S3-compatible endpoint** (path-style is fine). Alternatives to self-hosted Garage include **MinIO** (already used in repo `docker-compose` for local dev), **AWS S3**, **Cloudflare R2**, or any provider with an access key and bucket. Point DeployWerk’s storage settings at that endpoint instead of Garage if you prefer not to run Garage at all.

### Port conflicts

If **9292** is in use by **systemd** (Cockpit socket activation), the installer treats that as expected and continues with a warning. For a different process on 9292, set `COCKPIT_PORT` to another free port before running the installer.

If the installer aborts on a port conflict, identify the owner:

```bash
sudo ss -ltnp | grep -E ':80 |:443 |:8053 |:8080 |:9292 |:18080 |:3900 |:3902 |:3903 '
sudo ss -lunp | grep -E ':8053 '
docker ps --format 'table {{.Names}}\t{{.Ports}}'
```

The most common conflict is keeping native `deploywerk-api` on `localhost:8080` while also trying to bind Traefik’s local dashboard to the same port. The installer uses `localhost:18080` for the Traefik dashboard to avoid that.

Technitium DNS is published on **8053/tcp+udp** by default so it does not fight with typical host DNS listeners on **53** (for example `systemd-resolved` and libvirt `dnsmasq`). Clients must query **8053** unless you add your own forwarding from **53** to **8053**.

If you intentionally want Technitium on standard **53**, set `ENABLE_STANDARD_DNS_PORT_53=true` before running the installer (you will likely need to free or reconfigure host DNS services first).

## Optional: remote desktop / Hestia

XRDP and HestiaCP are **optional** and conflict with DeployWerk if they fight for the same **80/443** on one machine — keep separate hosts or one reverse proxy owner.

---

## Tight fit with your Docker stack (roadmap)

Use these in order; all exist or are stubbed in the current codebase:

- **`GET /api/v1/bootstrap`** — non-secret integration URLs for Traefik, Forgejo, Mailcow, Portainer, Technitium, Matrix client, etc.
- **`DEPLOYWERK_LOCAL_SERVICE_DEFAULTS`** — one-shot fill for typical single-host `localhost` ports when the API runs on the **host** (not inside Docker). With Compose, prefer explicit `DEPLOYWERK_INTEGRATION_*_URL` using `host.docker.internal` (see [.env.example](.env.example)).
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
export DEPLOYWERK_API_URL=http://localhost:8080
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
