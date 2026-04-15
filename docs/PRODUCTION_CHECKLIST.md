# Production checklist (DeployWerk)

This is a quick “go-live” checklist for a single DeployWerk host.

## DNS + TLS

- [ ] DNS `A/AAAA` records point to the server IP.
- [ ] TLS certificate issued (Let’s Encrypt) and auto-renew verified (`certbot renew --dry-run`).
- [ ] HTTP redirects to HTTPS.

## Ports / exposure

- [ ] Open inbound only what you need:
  - [ ] `22/tcp` (SSH)
  - [ ] `80/tcp`, `443/tcp` (nginx)
- [ ] Control panel (HestiaCP):
  - [ ] HestiaCP is deployed on a **separate host** (so it does not compete for 80/443 with DeployWerk).

## Docker (for future Platform Docker)

- [ ] Docker Engine installed and running (`docker version` works).
- [ ] You understand that `docker` group access is effectively root.
- [ ] `DEPLOYWERK_PLATFORM_DOCKER_ENABLED` remains `false` until you intentionally want platform Docker deploys on the API host.

## DeployWerk secrets and config

- [ ] `APP_ENV=production`
- [ ] `JWT_SECRET` is long random.
- [ ] `SERVER_KEY_ENCRYPTION_KEY` is set (32 bytes hex/base64).
- [ ] `/etc/deploywerk/deploywerk.env` has `0600` permissions and is backed up securely.

## Database

- [ ] Postgres is running and reachable on localhost.
- [ ] Backups configured (at least daily `pg_dump`) and stored off-host.
- [ ] Test restore procedure exists (even a minimal one).

## Email (optional)

- [ ] SMTP configured (`DEPLOYWERK_SMTP_*`) if you need invitations / email notifications.
- [ ] `DEPLOYWERK_PUBLIC_APP_URL` set to your public origin for email links.

## Deploy execution mode

- [ ] Decide: **inline** (API runs deploy tasks) vs **external worker** (`DEPLOYWERK_DEPLOY_DISPATCH=external`).
- [ ] If external:
  - [ ] `deploywerk-worker` systemd service enabled and healthy.

## Observability

- [ ] You can tail logs quickly:
  - [ ] `journalctl -u deploywerk-api -f`
  - [ ] `journalctl -u deploywerk-worker -f` (if enabled)
- [ ] Disk monitoring/alerts exist (DB + logs are common failure points).

## Health verification

- [ ] `curl -sf http://127.0.0.1:8080/api/v1/health`
- [ ] Browser loads the SPA and `/api/v1/bootstrap` succeeds (no 404).

