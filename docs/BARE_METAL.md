# Bare metal deployment (Ubuntu 24.04 LTS)

This guide is **step-by-step** for production-style self-hosting on **Ubuntu 24.04 LTS**, with **Let’s Encrypt TLS** and optional **XRDP**.

It covers **two hosts** so you never run two stacks that both want **TCP 80/443**:

| Host | Role | TLS |
|------|------|-----|
| **Host A** | DeployWerk (PostgreSQL, systemd API, **nginx** on 80/443) | **certbot** + nginx |
| **Host B** (optional) | Mail + Matrix + DNS (Docker Compose, **Traefik** on 80/443) | Traefik **ACME** (HTTP-01) |

Do **not** install Host A’s nginx and Host B’s Traefik on the same machine unless you redesign routing (one reverse proxy must own the public HTTP(S) ports).

```mermaid
flowchart LR
  subgraph hostA [HostA_Ubuntu]
    nginx[Nginx_80_443]
    api[DeployWerk_API]
    pg[(PostgreSQL)]
    nginx --> api
    api --> pg
  end
  subgraph hostB [HostB_Ubuntu_optional]
    traefik[Traefik_80_443]
    mail[Mailcow]
    matrix[Matrix]
    dns[Technitium]
    traefik --> mail
    traefik --> matrix
    traefik --> dns
  end
  users[Users] --> nginx
  users --> traefik
```

**Suggested order**

1. Harden SSH and firewall (both hosts).
2. **Host A:** Docker (optional, for future Platform Docker), DeployWerk, nginx, **certbot**, optional XRDP.
3. **Host B:** Docker, Traefik, Mailcow, Matrix, Technitium, DNS records, backups.

Replace placeholders such as `app.example.com`, `mail.example.com`, and `admin@example.com` with your real domains and contact email.

---

## 0) Hetzner Rescue → install Ubuntu 24.04

In the Rescue System, disks may be empty and changes may not persist until you install to disk.

1. Run:

```bash
installimage
```

2. Choose **Ubuntu 24.04 LTS**.
3. Partition disks (RAID1 recommended on dual-drive servers).
4. Reboot into the installed OS.

Repeat **per physical server** (Host A and Host B each get their own install if you use two machines).

---

## 1) First login: admin user + SSH hardening

On each host, create a sudo user (skip if your provider already did):

```bash
sudo adduser admin
sudo usermod -aG sudo admin
```

From your workstation, install your SSH key:

```bash
ssh-copy-id admin@<server-ip>
```

Edit `/etc/ssh/sshd_config` (use `sudo nano` or your editor):

- `PasswordAuthentication no`
- `PermitRootLogin no`

Reload SSH:

```bash
sudo systemctl reload ssh
```

Confirm you still have a working session before closing the root login.

---

## 2) Base packages, firewall, fail2ban

```bash
sudo apt update
sudo apt install -y ca-certificates curl git ufw fail2ban
```

### Host A — UFW (DeployWerk + optional XRDP)

Minimum inbound:

```bash
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
# Optional: remote desktop (restrict source IP in `ufw` or your cloud firewall when possible)
sudo ufw allow 3389/tcp
sudo ufw enable
sudo ufw status
```

### Host B — UFW (optional stack: mail, Matrix, DNS, XRDP)

Use this **instead** of the Host A rules on the **services** machine:

```bash
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Mailcow
sudo ufw allow 25/tcp
sudo ufw allow 465/tcp
sudo ufw allow 587/tcp
sudo ufw allow 110/tcp
sudo ufw allow 995/tcp
sudo ufw allow 143/tcp
sudo ufw allow 993/tcp
sudo ufw allow 4190/tcp

# Matrix federation (optional if you delegate federation to 443 only)
sudo ufw allow 8448/tcp

# XRDP
sudo ufw allow 3389/tcp

# Technitium DNS
sudo ufw allow 53/tcp
sudo ufw allow 53/udp

sudo ufw enable
sudo ufw status
```

### OS hygiene

- Enable unattended security upgrades and keep a reboot cadence.
- Confirm time sync (`timedatectl status`).

---

# Host A — DeployWerk

Perform the following on the **DeployWerk** server unless noted.

## A1) Base build dependencies, nginx, PostgreSQL

```bash
sudo apt update
sudo apt install -y \
  ca-certificates curl git \
  nginx \
  postgresql postgresql-contrib \
  build-essential pkg-config libssl-dev
```

### Node.js 22 (web UI build)

The repo’s `docker/Dockerfile.web` targets **Node 22**. Example via NodeSource:

```bash
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo bash -
sudo apt install -y nodejs
node -v
```

### Rust (cargo)

```bash
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
rustc -V
cargo -V
```

If you build as `admin`, ensure `source ~/.cargo/env` is available in non-interactive shells or use full paths; alternatively run builds in an interactive `bash -l` session.

## A2) Docker Engine (optional; future “Platform Docker”)

DeployWerk stays **native systemd binaries**. Docker is optional until you enable Platform Docker destinations.

**Security:** membership in the `docker` group is effectively **root**.

Install using Docker’s official Ubuntu guide: [Install Docker Engine on Ubuntu](https://docs.docker.com/engine/install/ubuntu/).

Then:

```bash
sudo systemctl enable --now docker
docker version
```

- Keep `DEPLOYWERK_PLATFORM_DOCKER_ENABLED=false` until you intentionally run deploy jobs on this host via local Docker.
- If you enable it, the runtime needs Docker socket access; understand the privilege risk.

## A3) Service user and directories

```bash
sudo useradd --system --create-home --home-dir /var/lib/deploywerk --shell /usr/sbin/nologin deploywerk

sudo mkdir -p /opt/deploywerk
sudo mkdir -p /var/lib/deploywerk/git-cache
sudo mkdir -p /var/lib/deploywerk/volumes
sudo mkdir -p /etc/deploywerk
sudo chown -R deploywerk:deploywerk /var/lib/deploywerk
```

Grant `deploywerk` read access to the checkout (after clone) with `chown -R deploywerk:deploywerk /opt/deploywerk` or group membership as you prefer.

## A4) PostgreSQL database and user

```bash
sudo -u postgres psql <<'SQL'
CREATE USER deploywerk WITH PASSWORD 'CHANGE_ME_STRONG_PASSWORD';
CREATE DATABASE deploywerk OWNER deploywerk;
SQL
```

## A5) DeployWerk environment file

Create `/etc/deploywerk/deploywerk.env` (`0600`). See repo root `.env.example` and `crates/deploywerk-api/src/config.rs` for all options.

```bash
sudo tee /etc/deploywerk/deploywerk.env >/dev/null <<'EOF'
APP_ENV=production

DATABASE_URL=postgresql://deploywerk:CHANGE_ME_STRONG_PASSWORD@127.0.0.1:5432/deploywerk

JWT_SECRET=CHANGE_ME_LONG_RANDOM
SERVER_KEY_ENCRYPTION_KEY=CHANGE_ME_32_BYTES_HEX_OR_BASE64

HOST=127.0.0.1
PORT=8080

DEPLOYWERK_PUBLIC_APP_URL=https://app.example.com

# Optional SMTP:
# DEPLOYWERK_SMTP_HOST=smtp.example.com
# DEPLOYWERK_SMTP_PORT=587
# DEPLOYWERK_SMTP_USER=
# DEPLOYWERK_SMTP_PASSWORD=
# DEPLOYWERK_SMTP_FROM=DeployWerk <noreply@example.com>
# DEPLOYWERK_SMTP_TLS=starttls

# Optional external worker:
# DEPLOYWERK_DEPLOY_DISPATCH=external
EOF

sudo chmod 600 /etc/deploywerk/deploywerk.env
```

Generate secrets:

```bash
openssl rand -base64 48    # JWT_SECRET
openssl rand -hex 32       # SERVER_KEY_ENCRYPTION_KEY
```

Edit the file with real values:

```bash
sudo nano /etc/deploywerk/deploywerk.env
```

## A6) Clone, build, install binaries

```bash
sudo git clone https://YOUR_GIT_REMOTE_HERE /opt/deploywerk
sudo chown -R "$USER:$USER" /opt/deploywerk
```

Build as the same user (needs Rust from §A1):

```bash
cd /opt/deploywerk
cargo build --release -p deploywerk-api --bin deploywerk-api
cargo build --release -p deploywerk-api --bin deploywerk-deploy-worker
```

Install binaries:

```bash
sudo install -m 0755 target/release/deploywerk-api /usr/local/bin/deploywerk-api
sudo install -m 0755 target/release/deploywerk-deploy-worker /usr/local/bin/deploywerk-deploy-worker
```

## A7) Web UI static files

```bash
cd /opt/deploywerk/web
npm ci
npm run build

sudo rm -rf /var/www/deploywerk
sudo mkdir -p /var/www/deploywerk
sudo cp -a dist/* /var/www/deploywerk/
```

Give the service user ownership of the repo and runtime dirs:

```bash
sudo chown -R deploywerk:deploywerk /opt/deploywerk
sudo chown -R deploywerk:deploywerk /var/lib/deploywerk
```

## A8) systemd units

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
RestartSec=2
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

If `DEPLOYWERK_DEPLOY_DISPATCH=external`, add `/etc/systemd/system/deploywerk-worker.service`:

```ini
[Unit]
Description=DeployWerk Deploy Worker
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
RestartSec=2
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now deploywerk-api
# sudo systemctl enable --now deploywerk-worker
```

Logs:

```bash
journalctl -u deploywerk-api -f
```

## A9) nginx (reverse proxy + SPA)

This mirrors the API/static split in `docker/nginx-web.conf`, but proxies to `127.0.0.1:8080`.

Create `/etc/nginx/sites-available/deploywerk.conf` (replace `app.example.com`):

```nginx
server {
  listen 80;
  server_name app.example.com;

  root /var/www/deploywerk;
  index index.html;

  location /api/ {
    proxy_pass http://127.0.0.1:8080;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_buffering off;
    proxy_read_timeout 86400s;
  }

  location / {
    try_files $uri $uri/ /index.html;
  }
}
```

Enable the site and reload:

```bash
sudo ln -sf /etc/nginx/sites-available/deploywerk.conf /etc/nginx/sites-enabled/deploywerk.conf
sudo nginx -t
sudo systemctl reload nginx
```

## A10) Let’s Encrypt TLS (certbot + nginx)

DNS **A/AAAA** for `app.example.com` must point to Host A before running certbot.

```bash
sudo apt install -y certbot python3-certbot-nginx
sudo certbot --nginx -d app.example.com
```

Certbot installs a systemd timer for renewal; verify with:

```bash
systemctl list-timers | grep certbot
```

## A11) Verify DeployWerk

On the server:

```bash
curl -sf http://127.0.0.1:8080/api/v1/health
curl -sf http://127.0.0.1:8080/api/v1/bootstrap | head
```

In a browser: `https://app.example.com/` — SPA loads and `/api/v1/bootstrap` is not 404.

## A12) Upgrades

```bash
sudo chown -R "$USER:$USER" /opt/deploywerk
cd /opt/deploywerk && git pull
cargo build --release -p deploywerk-api --bin deploywerk-api
cd web && npm ci && npm run build
sudo install -m 0755 /opt/deploywerk/target/release/deploywerk-api /usr/local/bin/deploywerk-api
sudo rm -rf /var/www/deploywerk/* && sudo cp -a /opt/deploywerk/web/dist/* /var/www/deploywerk/
sudo chown -R deploywerk:deploywerk /opt/deploywerk
sudo systemctl restart deploywerk-api
```

## A13) Host A — XRDP (Ubuntu desktop over RDP)

Ubuntu **Server** has no GUI by default. For a usable RDP session, install a lightweight desktop (example: **Xfce**):

```bash
sudo apt update
sudo apt install -y xubuntu-desktop
# Smaller alternative: sudo apt install -y xfce4 xfce4-goodies
```

Install and start **xrdp**:

```bash
sudo apt install -y xrdp
sudo adduser xrdp ssl-cert
sudo systemctl enable --now xrdp
sudo systemctl status xrdp --no-pager
```

Ensure UFW allows `3389/tcp` (see §2 Host A). Prefer restricting access to your IP or VPN in your cloud security group or:

```bash
# Example: only one IP (replace with yours)
# sudo ufw delete allow 3389/tcp
# sudo ufw allow from 203.0.113.50 to any port 3389 proto tcp
```

Connect with any RDP client to `your-server-ip:3389`, logging in with a Linux user that has a desktop session.

**Security:** exposing RDP to the internet is high risk; use VPN, SSH tunnel, or strict source IP filtering when possible.

## A14) Host A — notes

- **Platform Docker:** enabling local Docker deploys requires Docker and careful socket permissions for the `deploywerk` user (or a dedicated arrangement). Default is off.
- **Backups:** PostgreSQL dumps, `/etc/deploywerk/deploywerk.env`, and `/var/lib/deploywerk/volumes` if used.
- **Matrix:** prefer a separate host; see [MATRIX_FUTURE.md](MATRIX_FUTURE.md).

---

# Host B — Optional stack (Traefik, Mailcow, Matrix, Technitium)

Run this block on a **second** Ubuntu 24.04 server. Public email needs correct **DNS and PTR** before going live.

> Deliverability: set **reverse DNS (PTR)** for your sending IP to match `mail.example.com` (or your chosen hostname).

## B1) Docker Engine + Compose plugin

Follow [Install Docker Engine on Ubuntu](https://docs.docker.com/engine/install/ubuntu/).

```bash
sudo systemctl enable --now docker
docker version
docker compose version
sudo usermod -aG docker "$USER"
# newgrp docker   # or log out and back in
```

## B2) Traefik with Let’s Encrypt (HTTP-01)

```bash
sudo mkdir -p /opt/traefik/acme
sudo touch /opt/traefik/acme/acme.json
sudo chmod 600 /opt/traefik/acme/acme.json
sudo chown -R root:root /opt/traefik
```

Create `/opt/traefik/docker-compose.yml`. Set `admin@example.com` to your **Let’s Encrypt** contact email:

```yaml
services:
  traefik:
    image: traefik:v3.1
    command:
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
      - "--entrypoints.websecure.address=:443"
      - "--certificatesresolvers.le.acme.email=admin@example.com"
      - "--certificatesresolvers.le.acme.storage=/acme/acme.json"
      - "--certificatesresolvers.le.acme.httpchallenge=true"
      - "--certificatesresolvers.le.acme.httpchallenge.entrypoint=web"
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - "/var/run/docker.sock:/var/run/docker.sock:ro"
      - "./acme:/acme"
    networks:
      - proxy
    restart: unless-stopped

networks:
  proxy:
    name: proxy
    external: true
```

Start:

```bash
docker network create proxy || true
cd /opt/traefik
sudo docker compose up -d
sudo docker compose ps
```

Traefik obtains and renews certificates automatically via ACME.

## B3) Mailcow behind Traefik

Official Traefik v3 guide (routing, override, cert sync): [Mailcow + Traefik 3](https://docs.mailcow.email/post_installation/reverse-proxy/r_p-traefik3/).

```bash
cd /opt
sudo git clone https://github.com/mailcow/mailcow-dockerized
cd mailcow-dockerized
sudo ./generate_config.sh
```

In `mailcow.conf`, bind Mailcow HTTP(S) to localhost so Traefik owns 80/443:

```bash
HTTP_PORT=8080
HTTP_BIND=127.0.0.1
HTTPS_PORT=8443
HTTPS_BIND=127.0.0.1
SKIP_LETS_ENCRYPT=y
```

Add `docker-compose.override.yml` with Traefik labels and the `proxy` network per Mailcow’s documentation. Sync TLS material into Mailcow for SMTP/IMAP (e.g. `traefik-certs-dumper` → `./data/assets/ssl`).

```bash
sudo docker compose pull
sudo docker compose up -d
```

Verify `https://mail.example.com/` and mail ports from the internet.

## B4) Matrix (Synapse + Element) — skeleton

See [Synapse reverse proxy](https://matrix-org.github.io/synapse/latest/reverse_proxy.html). Replace `example.com` / `chat.example.com` below.

```bash
sudo mkdir -p /opt/matrix
sudo chown -R "$USER:$USER" /opt/matrix
cd /opt/matrix
```

Example `docker-compose.yml` (placeholder — adjust hosts so Element and Synapse do not collide on the same hostname):

```yaml
services:
  synapse-db:
    image: postgres:16
    environment:
      POSTGRES_DB: synapse
      POSTGRES_USER: synapse
      POSTGRES_PASSWORD: CHANGE_ME
    volumes:
      - synapse_db:/var/lib/postgresql/data
    restart: unless-stopped

  synapse:
    image: matrixdotorg/synapse:latest
    depends_on:
      - synapse-db
    environment:
      SYNAPSE_SERVER_NAME: "example.com"
      SYNAPSE_REPORT_STATS: "no"
    volumes:
      - ./synapse:/data
    networks:
      - proxy
      - default
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.synapse.rule=Host(`matrix.example.com`)"
      - "traefik.http.routers.synapse.entrypoints=websecure"
      - "traefik.http.routers.synapse.tls.certresolver=le"
      - "traefik.http.services.synapse.loadbalancer.server.port=8008"
    restart: unless-stopped

  element-web:
    image: vectorim/element-web:latest
    networks:
      - proxy
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.element.rule=Host(`chat.example.com`)"
      - "traefik.http.routers.element.entrypoints=websecure"
      - "traefik.http.routers.element.tls.certresolver=le"
      - "traefik.http.services.element.loadbalancer.server.port=80"
    restart: unless-stopped

networks:
  proxy:
    external: true

volumes:
  synapse_db:
```

**Federation:** open `8448/tcp` or use `/.well-known` delegation on 443.

**Calls:** plan **coturn**, and optionally Element Call + LiveKit; see [MATRIX_FUTURE.md](MATRIX_FUTURE.md).

## B5) Technitium DNS

Free port **53** if the container binds host `:53`:

```bash
sudo systemctl disable --now systemd-resolved
sudo rm -f /etc/resolv.conf
echo "nameserver 1.1.1.1" | sudo tee /etc/resolv.conf
```

```bash
sudo mkdir -p /opt/technitium
```

Create `/opt/technitium/docker-compose.yml`:

```yaml
services:
  technitium:
    image: technitium/dns-server:latest
    environment:
      DNS_SERVER_DOMAIN: "dns.example.com"
      DNS_SERVER_ADMIN_PASSWORD: "CHANGE_ME"
    ports:
      - "53:53/udp"
      - "53:53/tcp"
    volumes:
      - technitium_config:/etc/dns
    networks:
      - proxy
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.dnsui.rule=Host(`dns.example.com`)"
      - "traefik.http.routers.dnsui.entrypoints=websecure"
      - "traefik.http.routers.dnsui.tls.certresolver=le"
      - "traefik.http.services.dnsui.loadbalancer.server.port=5380"
    restart: unless-stopped

networks:
  proxy:
    external: true

volumes:
  technitium_config:
```

```bash
cd /opt/technitium
docker compose up -d
```

## B6) XRDP on Host B (optional)

Same steps as **§A13** if you want RDP on the services host. Ensure UFW allows `3389/tcp` (§2 Host B).

## B7) DNS checklist (mail)

For `mail.example.com` and zone `example.com`:

- **A/AAAA:** `mail.example.com` → Host B IP
- **MX:** `example.com` → `mail.example.com` (priority 10)
- **SPF** (TXT on `example.com`): e.g. `v=spf1 mx ~all` (tighten as needed)
- **DKIM:** TXT records from Mailcow
- **DMARC:** TXT on `_dmarc.example.com` (start `p=none`, then tighten)
- **PTR:** provider panel → reverse DNS for sending IP → `mail.example.com`

## B8) Backups

- Mailcow: `/opt/mailcow-dockerized` (especially `data/`), `mailcow.conf`
- Matrix: Postgres volume, `./synapse`, media store
- Technitium: volume
- Traefik: `/opt/traefik/acme/acme.json`

Store copies off-host.

## B9) Bootstrap script

Partial automation:

```bash
# From your workstation
scp scripts/server-bootstrap-orbytals.sh root@<server-ip>:/root/
```

On Host B:

```bash
chmod +x /root/server-bootstrap-orbytals.sh
sudo /root/server-bootstrap-orbytals.sh
```

Then finish Traefik compose, Mailcow overrides, Matrix config, and DNS per the sections above.

---

## Appendix: Hestia Control Panel (separate host)

HestiaCP expects to **own** web/mail/DNS on a machine. Do **not** install it on Host A or Host B alongside those stacks.

On a **fresh** server, follow [HestiaCP getting started](https://hestiacp.com/docs/introduction/getting-started). Example installer fetch:

```bash
sudo apt-get update && sudo apt-get install -y ca-certificates wget
wget https://raw.githubusercontent.com/hestiacp/hestiacp/release/install/hst-install.sh
sudo bash hst-install.sh
```

See `bash hst-install.sh -h` for options.

---

## Appendix: Matrix operations

For why Matrix is usually **not** co-located with DeployWerk, and ports/TURN/LiveKit notes, see [MATRIX_FUTURE.md](MATRIX_FUTURE.md).
