# DeployWerk — step-by-step server deployment

This guide walks through **production** on a Linux server: **PostgreSQL**, **Rust API** on loopback, **static SPA** + **`/api` reverse proxy** via **nginx** on another loopback port, and a **TLS edge** (Caddy, Traefik, nginx+certbot, aaPanel, etc.) in front.

The [README](../README.md) remains the primary operator reference (env summary, OIDC, mail, webhooks, troubleshooting). Authoritative env keys: [.env.example](../.env.example) and `crates/deploywerk-api/src/config.rs`.

---

## Architecture

```mermaid
flowchart LR
  subgraph edge [TLS_edge]
    CaddyOrNginx[Caddy_or_nginx_or_Traefik]
  end
  subgraph host [Host_loopback]
    NginxLoop[nginx_SPA_plus_api_proxy]
    Api[deploywerk_api_8080]
  end
  Pg[(PostgreSQL)]
  Browser[Browser] --> CaddyOrNginx
  CaddyOrNginx --> NginxLoop
  NginxLoop -->|"/api"| Api
  Api --> Pg
```

- **Edge** terminates HTTPS and forwards HTTP to **nginx** (e.g. `127.0.0.1:8085`).
- **nginx** serves files from `/var/www/deploywerk` and proxies `/api/` to **`127.0.0.1:8080`**.
- **deploywerk-api** listens only on loopback; it does not serve the React bundle.

---

## Prerequisites

- **OS:** Debian 12/13, Ubuntu 22.04/24.04, or similar (glibc, systemd).
- **DNS:** An `A` (and optionally `AAAA`) record for your public hostname pointing at the server.
- **Firewall:** Allow **22** (SSH), **80** and **443** (HTTPS and ACME HTTP-01 if you use Let’s Encrypt HTTP challenge). Open extra ports only for services you run (mail, Matrix, etc.).
- **Toolchains:** [Rust via rustup](https://rustup.rs/), **Node.js 22** ([NodeSource](https://github.com/nodesource/distributions) or `nvm`) for `web/` builds.
- **Docker:** Install only if you use **Platform Docker** deploys (`DEPLOYWERK_PLATFORM_DOCKER_ENABLED=true`). The `docker` group is effectively root — grant it only to `deploywerk` if you need it.

Base packages:

```bash
sudo apt update
sudo apt install -y nginx postgresql build-essential pkg-config libssl-dev git curl
```

---

## Step 1 — Service user and directories

```bash
sudo useradd --system --create-home --home-dir /var/lib/deploywerk --shell /usr/sbin/nologin deploywerk
sudo mkdir -p /opt/deploywerk /var/lib/deploywerk/git-cache /var/lib/deploywerk/volumes /etc/deploywerk /var/www/deploywerk
sudo chown -R deploywerk:deploywerk /var/lib/deploywerk
```

Clone the repository to `/opt/deploywerk` (as `deploywerk` or your admin user, then fix ownership):

```bash
sudo mkdir -p /opt/deploywerk
sudo chown "$USER:$USER" /opt/deploywerk
cd /opt
git clone https://github.com/chavanashutosh/DeployWerk_V2.git deploywerk
sudo chown -R deploywerk:deploywerk /opt/deploywerk
```

---

## Step 2 — PostgreSQL

### Option A — Native PostgreSQL on the host

Create a dedicated role and database (replace `STRONG_PASSWORD`):

```bash
sudo -u postgres psql -c "CREATE USER deploywerk WITH PASSWORD 'STRONG_PASSWORD';"
sudo -u postgres psql -c "CREATE DATABASE deploywerk OWNER deploywerk;"
```

Example `DATABASE_URL` for the env file:

```text
postgresql://deploywerk:STRONG_PASSWORD@127.0.0.1:5432/deploywerk
```

### Option B — Dedicated Postgres in Docker (from this repo)

Use this when you want an isolated Postgres container and to avoid host port clashes with another PostgreSQL or stacks (for example a different container already using **15432** on the host). From the repo root:

```bash
docker compose up -d postgres
```

By default, [docker-compose.yml](../docker-compose.yml) publishes **`${DEPLOYWERK_POSTGRES_HOST_PORT:-15433}:5432`**. Point **`DATABASE_URL`** at that **host** port (credentials default to `deploywerk` / `deploywerk` unless you change `POSTGRES_*` in Compose):

```text
postgresql://deploywerk:deploywerk@127.0.0.1:15433/deploywerk
```

If **15433** is taken, set `DEPLOYWERK_POSTGRES_HOST_PORT` (e.g. in `.env`) and use the same port in `DATABASE_URL`. Processes **inside** Compose still use hostname `postgres` and port **5432**; only the host API and `/etc/deploywerk/deploywerk.env` use `127.0.0.1` and the published port.

---

## Step 3 — Environment file

Create `/etc/deploywerk/deploywerk.env` and restrict permissions:

```bash
sudo install -m 600 /dev/null /etc/deploywerk/deploywerk.env
sudo chown root:deploywerk /etc/deploywerk/deploywerk.env
```

**Minimal production content** (adjust URLs to your real hostname):

```bash
APP_ENV=production
SEED_DEMO_USERS=false
DEMO_LOGINS_PUBLIC=false

HOST=127.0.0.1
PORT=8080
RUST_LOG=deploywerk_api=info,tower_http=info,sqlx=warn

DATABASE_URL=postgresql://deploywerk:STRONG_PASSWORD@127.0.0.1:5432/deploywerk
# If using Docker Compose postgres (Step 2 option B), use 127.0.0.1:15433 and matching credentials.
DATABASE_MAX_CONNECTIONS=20

JWT_SECRET=REPLACE_WITH_openssl_rand_-base64_48
SERVER_KEY_ENCRYPTION_KEY=REPLACE_WITH_openssl_rand_-hex_32

DEPLOYWERK_PUBLIC_APP_URL=https://app.example.com
DEPLOYWERK_API_URL=https://app.example.com

DEPLOYWERK_GIT_CACHE_ROOT=/var/lib/deploywerk/git-cache
DEPLOYWERK_VOLUMES_ROOT=/var/lib/deploywerk/volumes

DEPLOYWERK_DEPLOY_DISPATCH=inline
```

Generate secrets:

```bash
openssl rand -base64 48   # JWT_SECRET
openssl rand -hex 32      # SERVER_KEY_ENCRYPTION_KEY
```

**Same-origin browser → nginx → API:** use one public URL for both `DEPLOYWERK_PUBLIC_APP_URL` and `DEPLOYWERK_API_URL` when nginx serves the SPA and `/api`. Build the SPA **without** embedding another API origin (leave `VITE_API_URL` unset in the build environment so the UI uses relative `/api`).

Add storage (`DEPLOYWERK_DEFAULT_STORAGE_*`), SMTP (`DEPLOYWERK_SMTP_*`), OIDC (`AUTHENTIK_*`), Platform Docker, and integration URLs as needed — see [.env.example](../.env.example).

---

## Step 4 — Build and install binaries

As `deploywerk` from `/opt/deploywerk`:

```bash
sudo -u deploywerk -H bash -lc 'cd /opt/deploywerk && cargo build --release -p deploywerk-api --bin deploywerk-api'
sudo -u deploywerk -H bash -lc 'cd /opt/deploywerk && cargo build --release -p deploywerk-api --bin deploywerk-deploy-worker'
sudo install -m 0755 /opt/deploywerk/target/release/deploywerk-api /usr/local/bin/deploywerk-api
sudo install -m 0755 /opt/deploywerk/target/release/deploywerk-deploy-worker /usr/local/bin/deploywerk-deploy-worker
```

---

## Step 5 — Build and install the web UI

Build with production env so `VITE_*` and public URLs match your deployment (export vars inline or use a small env file for the build only):

```bash
sudo -u deploywerk -H bash -lc 'cd /opt/deploywerk/web && npm ci && npm run build'
sudo rsync -a --delete /opt/deploywerk/web/dist/ /var/www/deploywerk/
# or: sudo cp -a /opt/deploywerk/web/dist/. /var/www/deploywerk/
sudo chown -R deploywerk:deploywerk /var/www/deploywerk
```

---

## Step 6 — systemd units

### `deploywerk-api.service`

Create `/etc/systemd/system/deploywerk-api.service`:

```ini
[Unit]
Description=DeployWerk API
After=network-online.target postgresql.service
Wants=network-online.target

[Service]
Type=simple
User=deploywerk
Group=deploywerk
WorkingDirectory=/opt/deploywerk
EnvironmentFile=/etc/deploywerk/deploywerk.env
ExecStart=/usr/local/bin/deploywerk-api
Restart=on-failure
RestartSec=5

# Hardening (optional; relax if something legitimate breaks)
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

### `deploywerk-deploy-worker.service` (only if `DEPLOYWERK_DEPLOY_DISPATCH=external`)

Match the same `User`, `WorkingDirectory`, and `EnvironmentFile`:

```ini
[Unit]
Description=DeployWerk deploy worker
After=network-online.target postgresql.service deploywerk-api.service
Wants=network-online.target

[Service]
Type=simple
User=deploywerk
Group=deploywerk
WorkingDirectory=/opt/deploywerk
EnvironmentFile=/etc/deploywerk/deploywerk.env
ExecStart=/usr/local/bin/deploywerk-deploy-worker
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now deploywerk-api
sudo systemctl status deploywerk-api --no-pager
journalctl -u deploywerk-api -f
```

If using the worker: `sudo systemctl enable --now deploywerk-deploy-worker`.

---

## Step 7 — nginx (loopback SPA + `/api`)

**Alternative without a system nginx site:** From the repo, [scripts/deploywerk-caddy.sh](../scripts/deploywerk-caddy.sh) runs **deploywerk-api** and a dedicated nginx on loopback (`sudo bash scripts/deploywerk-caddy.sh start` — state and PID files under `DEPLOYWERK_STATE_DIR`, default `/var/lib/deploywerk/run`). Use **`stop`** / **`status`** / **`restart`** as needed; **`run`** keeps nginx in the foreground for debugging. Point **Caddy** (or another edge) at that HTTP port; issue **Let’s Encrypt** certificates on the edge, not inside this script. **`caddy-snippet`** prints a minimal `reverse_proxy` block.

Pick a **loopback** port for nginx (examples use **8085**; your edge proxy must match). Create `/etc/nginx/sites-available/deploywerk.conf`:

```nginx
server {
    listen 127.0.0.1:8085;
    server_name _;

    root /var/www/deploywerk;
    index index.html;

    location /api/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-Host $host;
        proxy_buffering off;
        proxy_read_timeout 86400s;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

When TLS is **in front** of nginx (Caddy/Traefik), set **`X-Forwarded-Proto`** correctly. If the inner hop is plain HTTP and `$scheme` is `http`, force the header the API sees:

```nginx
proxy_set_header X-Forwarded-Proto https;
```

Enable the site and reload:

```bash
sudo ln -sf /etc/nginx/sites-available/deploywerk.conf /etc/nginx/sites-enabled/deploywerk.conf
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t && sudo systemctl reload nginx
```

**Docker → host:** If the edge runs in Docker and reaches the host via the bridge gateway (often `172.17.0.1`), nginx must **listen** on that address as well, or the edge must target an IP where nginx listens. See [README — Traefik on the same host](../README.md#traefik-on-the-same-host-deploywerk-native-edge-in-docker).

---

## Step 8 — TLS and public hostname

Choose **one** pattern.

### A. Reverse proxy terminates TLS (Caddy, Traefik, HAProxy, cloud LB)

- Bind nginx to **`127.0.0.1:8085`** (or another loopback port).
- Configure the edge: `reverse_proxy` / router to `http://127.0.0.1:8085` for your hostname.
- Preserve **Host** and **X-Forwarded-Proto https** so OAuth and redirects work.

### B. nginx terminates TLS on this server

- Change `listen` to `443 ssl` with certificate paths, or use **certbot** (`python3-certbot-nginx`) after DNS is correct.
- Point `DEPLOYWERK_PUBLIC_APP_URL` / `DEPLOYWERK_API_URL` at `https://…`.

### C. aaPanel (or similar panel)

Use the panel’s **site** + **reverse proxy** to forward to **`http://127.0.0.1:8085`** (or split **static** + **`/api`** to `8080` if you configure two upstreams). Panel Let’s Encrypt can cover public DNS. See [README — Production with aaPanel](../README.md#production-with-aapanel-control-panel) and [scripts/orbytals-install.sh](../scripts/orbytals-install.sh) for aaPanel install hints only.

---

## Step 9 — Verification

```bash
curl -sf http://127.0.0.1:8080/api/v1/health
curl -sf http://127.0.0.1:8080/api/v1/bootstrap | head
curl -sI http://127.0.0.1:8085/ | head -5
curl -sI http://127.0.0.1:8085/api/v1/health | head -5
```

From your workstation (public DNS):

- Open `https://your-hostname/` — SPA loads.
- `https://your-hostname/api/v1/bootstrap` returns JSON (not 404).

---

## Optional next steps

| Topic | Where to read |
|--------|----------------|
| OIDC (Authentik-shaped vars) | [README — Single sign-on (OIDC)](../README.md#single-sign-on-oidc) |
| SMTP / Mail | [README — Mail](../README.md#mail) |
| Git / GitHub / GitLab webhooks | [README — Webhooks](../README.md#webhooks-reference) |
| Platform Docker + Traefik labels | [.env.example](../.env.example), README env table |
| Production checklist | [README — Production checklist](../README.md#production-checklist) |

---

## Upgrades

```bash
sudo systemctl stop deploywerk-api
sudo -u deploywerk -H bash -lc 'cd /opt/deploywerk && git pull && cargo build --release -p deploywerk-api --bin deploywerk-api --bin deploywerk-deploy-worker'
sudo install -m 0755 /opt/deploywerk/target/release/deploywerk-api /usr/local/bin/deploywerk-api
sudo install -m 0755 /opt/deploywerk/target/release/deploywerk-deploy-worker /usr/local/bin/deploywerk-deploy-worker
sudo -u deploywerk -H bash -lc 'cd /opt/deploywerk/web && npm ci && npm run build'
sudo rsync -a --delete /opt/deploywerk/web/dist/ /var/www/deploywerk/
sudo systemctl start deploywerk-api
```

---

## Troubleshooting

- **502 on `/api`:** API not running, wrong `proxy_pass`, or nginx cannot reach `127.0.0.1:8080`.
- **SPA works but API 404:** nginx `location /api/` or trailing slash on `proxy_pass` mismatch; see [docker/nginx-web.conf](../docker/nginx-web.conf).
- **curl from the server to its own public hostname fails:** hairpin NAT; test with `curl --resolve your.domain:443:127.0.0.1 https://your.domain/` or use loopback checks above. See [README — Troubleshooting (production)](../README.md#troubleshooting-production).

For more, use the [README](../README.md) index and search.
