# DeployWerk V2

Early **Rust** workspace (HTTP API + CLI) with a **Vite + React + Tailwind** web UI. Product scope and phased delivery are driven by [docs/USE_CASES_AND_SCENARIOS.md](docs/USE_CASES_AND_SCENARIOS.md), derived from the DeployWerk functionality reference.

## What is implemented today

- **API** (`deploywerk-api`): health/version, registration and login (JWT), current user, team list, bootstrap endpoint for demo login visibility, SQLite persistence via `sqlx` migrations.
- **Web** (`web/`): public marketing shell, pricing placeholder, legal placeholders, sign-in / register, dashboard shell with team list, sample logins page (reads bootstrap).
- **CLI** (`deploywerk-cli`, binary `deploywerk`): `auth login`, `teams list`.
- **Core** (`deploywerk-core`): shared IDs and team role types.

Typography uses **Plus Jakarta Sans** (Google Fonts) as a redistributable alternative to proprietary Google Sans. Icons use **Lucide** (line style). UI patterns follow **HyperUI**-style Tailwind cards and layout primitives.

## Prerequisites

- Rust toolchain (2021 edition)
- Node.js 20+ and npm (for the web app)

## Configuration

Copy [.env.example](.env.example) to `.env` in the **repository root** when running the API from that directory.

| Variable | Purpose |
|----------|---------|
| `DATABASE_URL` | SQLite connection string (default `sqlite:deploywerk.db?mode=rwc`) |
| `JWT_SECRET` | Symmetric key for JWTs (set a strong value in production) |
| `APP_ENV` | `development` or `production` — production disables demo seeding and suppresses demo password exposure |
| `SEED_DEMO_USERS` | When true (and not production), seeds demo team and users |
| `DEMO_LOGINS_PUBLIC` | When true (and not production), `GET /api/v1/bootstrap` returns demo passwords |
| `HOST` / `PORT` | API bind address |

### Demo accounts (development only)

When `SEED_DEMO_USERS` is enabled (default in non-production), the API creates a **Demo Team** and:

| Email | Role | Password |
|-------|------|----------|
| `owner@demo.deploywerk.local` | owner | `DemoOwner1!` |
| `admin@demo.deploywerk.local` | admin | `DemoAdmin1!` |
| `member@demo.deploywerk.local` | member | `DemoMember1!` |

**Never enable demo seeding or `DEMO_LOGINS_PUBLIC` in production.** With `APP_ENV=production`, both are forced off regardless of flags.

## Run the API

From the repository root:

```bash
cargo run -p deploywerk-api
```

The database file and migrations apply on startup. API listens on `http://127.0.0.1:8080` by default.

### API quick reference

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/health` | No | Liveness |
| GET | `/api/v1/version` | No | Version string |
| GET | `/api/v1/bootstrap` | No | Demo flags / accounts |
| POST | `/api/v1/auth/register` | No | Create user + default team |
| POST | `/api/v1/auth/login` | No | JWT |
| GET | `/api/v1/me` | Bearer | Current user |
| GET | `/api/v1/teams` | Bearer | Team memberships |

## Run the web UI

```bash
cd web
npm install
npm run dev
```

Vite serves on `http://127.0.0.1:5173` and proxies `/api` to the API on port 8080.

Production build:

```bash
cd web
npm run build
```

Serve `web/dist` with any static host; the app expects the same origin to serve `/api` or configure a reverse proxy.

## CLI

Build and install from the workspace:

```bash
cargo install --path crates/deploywerk-cli
```

Or run without installing:

```bash
cargo run -p deploywerk-cli -- auth login --email you@example.com
cargo run -p deploywerk-cli -- teams list
```

Config and token path (per OS): see the `directories` crate — typically `deploywerk-cli` under the OS config directory (`dev.deploywerk.deploywerk-cli`).

Override API URL:

```bash
deploywerk --base-url http://127.0.0.1:8080 teams list
```

## Workspace layout

```
crates/deploywerk-core   # Shared types
crates/deploywerk-api    # Axum HTTP API + SQLx migrations
crates/deploywerk-cli    # Command-line client
web/                     # React + Vite + Tailwind frontend
docs/USE_CASES_AND_SCENARIOS.md
```

## Build all Rust crates

```bash
cargo build --workspace
```

## License

MIT (see crate `Cargo.toml` workspace metadata).
