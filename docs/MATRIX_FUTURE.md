# Matrix (future) deployment plan

This document outlines a future plan to deploy a **Matrix** chat stack in a way that stays compatible with a production DeployWerk install.

## Recommendation: separate host

Even if you have a large bare metal box, Matrix is operationally distinct (federation, media store growth, TURN/VoIP, different upgrade cadence). For production-grade operations, prefer:

- **Host A**: DeployWerk (as per [BARE_METAL.md](BARE_METAL.md), Host A)
- **Host B**: Matrix stack (this doc)

This avoids nginx/port/config “ownership” conflicts and makes incident response simpler.

**Constraint alignment:** DeployWerk should stay **native** (systemd, not Docker). Matrix can be deployed on Host B using **Docker Compose** if you prefer container operations there.

## Components (baseline)

- **Matrix homeserver**: Synapse (most common) + **PostgreSQL** (recommended for production)
- **Web client**: Element Web
- **VoIP/Video connectivity**: coturn (TURN server)
- **Group calls (recommended)**: Element Call + LiveKit (SFU)
- **Reverse proxy / TLS**: nginx (Traefik is optional, not required)

## Domain model

Common, low-surprise approach:

- `matrix.example.com` → Synapse (client-server API via reverse proxy on 443)
- `element.example.com` → Element Web (static)
- `turn.example.com` → coturn (TURN on 3478/5349)

You also need Matrix discovery endpoints:

- `https://<base>/.well-known/matrix/client`
- `https://<base>/.well-known/matrix/server`

These can be served from your main “base domain” (often `example.com`) even if Synapse runs on `matrix.example.com`.

## Ports

Minimum you should plan for:

- **Client access**: `443/tcp` to your reverse proxy
- **Federation**: `8448/tcp` (Synapse) unless you deliberately disable federation
- **TURN**: `3478/udp` and `3478/tcp` (and/or `5349/tcp` for TURN/TLS) + a UDP relay port range (per coturn config)

For Element Call + LiveKit you will also need:

- **LiveKit signaling**: `7881/tcp` (commonly used for WebSocket signaling)
- **LiveKit media**: `7882/udp` (or a configured UDP port/range)

## Reverse proxy requirements (nginx)

Matrix Synapse has specific reverse-proxy requirements (headers, URI handling, upload sizes). Use the official Synapse reverse proxy guide as the source of truth:

- `https://matrix-org.github.io/synapse/latest/reverse_proxy.html`

Minimum notes to carry into your deployment:

- Forward `X-Forwarded-For` and `X-Forwarded-Proto`.
- Ensure your proxy does not “normalize” or rewrite URIs in a way that breaks signature verification.
- Set nginx `client_max_body_size` to match Synapse `max_upload_size`.

## coturn notes

- coturn is needed for reliable calls when clients are behind NAT.
- Use Synapse TURN shared secret integration where possible (`turn_shared_secret`), and ensure both UDP and TCP work from typical client networks.

## Audio/Video calls (production): Element Call + LiveKit

For modern Matrix audio/video calling, plan for:

- **Element Call**: the web-based calling UI (usually embedded as a widget or accessed via a dedicated URL)
- **LiveKit**: the SFU that handles multi-party call media routing
- **MatrixRTC auth (lk-jwt-service)**: issues JWTs so clients can access LiveKit securely (required by the Element Call + LiveKit approach)
- **coturn**: still recommended for NAT traversal and difficult networks

Client discovery / configuration (high-level):

- Publish the appropriate MatrixRTC configuration in `/.well-known/matrix/client` so clients know where the RTC backend lives.
- Follow Element’s self-hosting documentation for the exact keys/MSC requirements for your homeserver and clients.

References:

- Element Call self-hosting (LiveKit): `https://github.com/element-hq/element-call/blob/livekit/docs/self-hosting.md`
- Element docs: `https://docs.element.io/latest/element-server-suite-classic/integrations/setting-up-element-call/`

## Data and backups

- **Postgres**: nightly backups, tested restore.
- **Media store**: plan for growth (disk) and backup strategy (or offload media to object storage if you later adopt that model).

## Traefik: when it helps vs when it’s unnecessary

Traefik is **not required** for Matrix. Choose it if:

- You want a container-first stack with automatic routing rules and cert management.

Stick with nginx if:

- You already standardize on nginx for other services and want fewer moving parts.


