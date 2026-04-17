# Orbytals install verification (`scripts/orbytals-install.sh all`)

After `sudo bash scripts/orbytals-install.sh all`, the script runs **verify** steps: port summary, HTTPS checks through Traefik on `127.0.0.1:443` with `--resolve` (SNI), then loopback HTTP checks for DeployWerk and Garage.

## How to read the results

| Check | Healthy sign | Your run (example) |
|--------|----------------|-------------------|
| `http://127.0.0.1:8085` | `HTTP/1.1 200` from nginx | OK |
| `http://127.0.0.1:8080/api/v1/health` | `HTTP/1.1 200` JSON | OK |
| `http://127.0.0.1:3900` (Garage) | Any HTTP (often `403` on `/`) or TCP open | OK (`403` is normal for unauthenticated S3 root) |
| `https://traefik.<domain>/` | `401` with `www-authenticate: Basic` (dashboard auth) | OK |
| `https://app.<domain>/` via Traefik | Any HTTP response within timeout (ideally `200` / `304`) | **Timeout** → see below |
| `https://api.<domain>/api/v1/bootstrap` | Same | **Timeout** → same root cause |
| `https://cockpit.<domain>/` | Same | **Timeout** → UFW + Traefik→host |
| `https://mail.<domain>/` etc. | `200`–`499` counts as “Traefik answered” | `404` → Traefik reached default/no router or backend; still worth tuning |

The verify helper treats **any** HTTP status line as success for Traefik HTTPS checks (including `404`), because it proves Traefik terminated TLS and routed somewhere. **Timeouts** mean no HTTP response (wrong upstream, firewall drop, or upstream not listening on the address Traefik uses).

## Ports to allow (firewall / security groups)

The installer configures **UFW** on the host. If you also use a **cloud firewall** (Hetzner Firewall, AWS security groups, etc.), mirror the same **inbound** rules there; otherwise traffic can be dropped before it reaches UFW.

### Inbound from the Internet (typical defaults)

| Port(s) | Protocol | Purpose |
|---------|----------|---------|
| **22** | TCP | SSH |
| **80** | TCP | HTTP (Traefik, Let’s Encrypt HTTP-01) |
| **443** | TCP | HTTPS (Traefik, all public web / API hostnames) |
| **2222** | TCP | Forgejo Git over SSH (`FORGEJO_SSH_PORT`, default `2222`) |

When **`ENABLE_PUBLIC_MAIL_PORTS=true`** (default in `orbytals-install.sh`):

| Port(s) | Protocol | Purpose |
|---------|----------|---------|
| **25, 465, 587** | TCP | SMTP / submission (Mailcow via Traefik) |
| **110, 995** | TCP | POP3 / POP3S |
| **143, 993** | TCP | IMAP / IMAPS |
| **4190** | TCP | ManageSieve |

When **`ENABLE_PUBLIC_DNS_PORTS=true`** (default):

| Port(s) | Protocol | Purpose |
|---------|----------|---------|
| **`8053`** (default `TECHNITIUM_DNS_PORT`) | TCP **and** UDP | Technitium DNS (published as host `8053` → container `53`) |

When **`ENABLE_STANDARD_DNS_PORT_53=true`** (default **false**):

| Port(s) | Protocol | Purpose |
|---------|----------|---------|
| **53** | TCP **and** UDP | DNS on the standard port |

When **`ENABLE_PUBLIC_MATRIX_FEDERATION_PORT=true`** (default):

| Port(s) | Protocol | Purpose |
|---------|----------|---------|
| **8448** | TCP | Matrix federation (Synapse) |

When **`OPEN_COCKPIT_PORT=true`** (default **false**):

| Port(s) | Protocol | Purpose |
|---------|----------|---------|
| **`9292`** (default `COCKPIT_PORT`) | TCP | Cockpit exposed on the public interface (not only via Traefik) |

### Inbound from Docker private networks only (host UFW)

These rules allow **containers** (e.g. Traefik) to reach **services bound on the host**. They use **`ufw allow from 172.16.0.0/12`** (RFC1918 range used by default Docker networks). You normally **do not** open these ports to `0.0.0.0/0` on a cloud firewall.

| Port (default) | Protocol | Purpose |
|----------------|----------|---------|
| **8085** (`DEPLOYWERK_NGINX_PORT`) | TCP | DeployWerk nginx (Traefik → host SPA + `/api/` proxy) |
| **9292** | TCP | Cockpit on the host when **`OPEN_COCKPIT_PORT=false`** (Traefik → Cockpit only from Docker, not from the whole Internet) |

### Loopback-only (no WAN rule)

These are bound to **`127.0.0.1`** (or otherwise not exposed on `0.0.0.0`) in the default layout; **do not** need to be opened on a perimeter firewall:

| Port (default) | Purpose |
|----------------|---------|
| **18080** (`TRAEFIK_DASHBOARD_LOCAL_PORT`) | Traefik dashboard (container port mapped to loopback) |
| **8080** (`DEPLOYWERK_API_PORT`) | DeployWerk API (systemd + nginx `proxy_pass`) |
| **8082** / **8444** | Mailcow HTTP/HTTPS binds (behind Traefik) |
| **3900**, **3902**, **3903** | Garage S3 / web / admin (host loopback) |
| **5380** (`TECHNITIUM_HTTP_PORT`) | Technitium web UI (loopback; Traefik uses Docker network to the container) |
| **5432** | PostgreSQL (local only in installer flow) |

**Garage RPC** default **3901** is used between Garage peers; single-node install keeps it on the container network. Adjust if you change `GARAGE_*` or Traefik/Mailcow env vars in the script.

### Summary checklist for a public edge server

1. **WAN:** **22**, **80**, **443**, **2222** + mail/DNS/Matrix toggles as in the tables above.  
2. **Cloud SG:** match UFW; avoid exposing **8080**, **8085**, **9292** to the world unless you intend to (the installer keeps API/nginx/Cockpit off the public interface by default, except **9292** when `OPEN_COCKPIT_PORT=true`).  
3. **Docker → host:** ensure UFW (or equivalent) allows **`172.16.0.0/12` → 8085** and **`172.16.0.0/12` → 9292** when Cockpit is not public—see the next section.

## Why `app` / `api` / `cockpit` timed out (common on Linux)

1. **DeployWerk nginx** was only listening on **`127.0.0.1:8085`**, while Traefik (in Docker) calls the host using the **Docker bridge gateway** (often `172.17.0.1`). Traffic to `172.17.0.1:8085` never hit a listening socket → packets dropped or hanging → **curl timeout**.

2. **UFW** default **deny incoming** blocks Docker → host ports unless allowed. A previous **`ufw deny` on the Cockpit port** also blocked Traefik → Cockpit on the host.

The installer was updated to:

- add a second nginx **`listen`** on `$(docker network inspect bridge … Gateway):8085` when that IP is not loopback;
- **`ufw allow from 172.16.0.0/12`** to the DeployWerk nginx port and (when Cockpit is not opened to the world) to the Cockpit port, instead of blanket **`ufw deny`** on Cockpit.

Re-run **`install_native_deploywerk`** (full `all` is fine) or at least re-apply nginx + UFW from the updated script, then run **`sudo bash scripts/orbytals-install.sh verify`**.

## `404` from Traefik for mail / git / dns / Matrix

An HTTP/2 `404` with a small body usually means **Traefik handled the request** but no router matched, or the backend returned `404` for that path. Typical follow-ups:

- Confirm DNS **A/AAAA** for those names points at this server (optional for loopback verify; required from the Internet).
- Let **ACME** finish (`acme.json`); the script may print “TLS certificate verify skipped” while using `-k` — that is expected until the public chain is valid.
- For Matrix, **`.well-known`** and federation often need extra routes on the **apex** domain; the bundled file provider only wires what the template describes.

## `install.env` and `apt` (from earlier runs)

- State file must be valid **`KEY=value`** lines; spaces immediately after **`=`** are unsafe when the file is sourced (see script sanitizer and README troubleshooting).
- **`apt update`** mirror hash/size errors are usually transient; the installer retries `apt-get update`.

## Quick manual checks (on the server)

```bash
# Traefik still routing after fixes
curl -sSI --max-time 15 --resolve "app.orbytals.com:443:127.0.0.1" https://app.orbytals.com/

# Nginx listens where Traefik expects (example: 172.17.0.1 = docker0 gateway)
ss -ltnp | grep ':8085'

# UFW rules mentioning DeployWerk / Cockpit / 172.16
sudo ufw status numbered | head -40
```

If anything still fails, capture **`docker logs traefik --tail 200`** and **`curl -v`** for one failing host.
