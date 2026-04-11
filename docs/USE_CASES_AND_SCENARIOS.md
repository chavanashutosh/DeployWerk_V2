# DeployWerk — Use cases and user scenarios

This document turns the product functionality reference into **implementation-oriented scenarios**: actors, goals, preconditions, main flows, extensions, data touched, and suggested delivery **phase** tags. Update **API/UI touchpoints** as endpoints and pages land.

## Conventions

- **Phases**: `P0` (platform shell), `P1` (tenancy + auth + teams/projects), `P2` (servers + destinations), `P3` (applications + deploy worker), `P4` (databases, services, backups), `P5` (notifications, billing, integrations), `P6` (advanced: swarm, sentinel, tunnels).
- **Actors**: Operator (instance admin), OrgOwner, TeamOwner, TeamAdmin, TeamMember, ApiClient (token).

---

## Part A — Tenancy structure

### UC-A1: User switches team context

| Field | Content |
|--------|---------|
| **Goal** | Member works in the correct team workspace. |
| **Actors** | TeamMember, TeamAdmin |
| **Preconditions** | User belongs to ≥1 team; session established. |
| **Main success** | 1) User opens team switcher. 2) Selects team T. 3) UI and API requests use T’s scope. 4) Listed servers, projects, resources reflect T. |
| **Extensions** | Only one team → switcher hidden. Last team remembered. |
| **Data** | `user`, `team_membership`, `current_team_id` (session) |
| **Phase** | P1 |
| **API/UI** | `GET /api/v1/teams`, `POST /api/v1/session/team` (future); settings UI |

### UC-A2: Admin creates project and environments

| Field | Content |
|--------|---------|
| **Goal** | Isolate prod vs staging (etc.) under one product. |
| **Actors** | TeamAdmin, TeamOwner |
| **Preconditions** | Create/update permission on team; team selected. |
| **Main success** | 1) Create project P. 2) Create env `production`, `staging`. 3) Resource wizards target an environment. |
| **Extensions** | Clone environment; delete blocked if resources exist (policy). |
| **Data** | `project`, `environment` under `team` |
| **Phase** | P1 |
| **API/UI** | Projects CRUD, environments CRUD |

### UC-A3: Place resource on server destination

| Field | Content |
|--------|---------|
| **Goal** | Running workloads land on the intended Docker context. |
| **Actors** | TeamMember (if allowed), TeamAdmin |
| **Preconditions** | Server and destination exist; user has update on resource. |
| **Main success** | 1) Open resource → Servers/destination. 2) Pick server S, destination D. 3) Save; next deploy targets D. |
| **Extensions** | Server unreachable warning; swarm vs standalone destination. |
| **Data** | `server`, `destination`, resource placement FKs |
| **Phase** | P2 |
| **API/UI** | Server list, destination list, resource update |

---

## Part B — Roles and permissions

### UC-B1: Member denied destructive action

| Field | Content |
|--------|---------|
| **Goal** | Enforce least privilege. |
| **Actors** | TeamMember |
| **Preconditions** | Member lacks delete on target. |
| **Main success** | Delete (or destructive) action returns 403 / inline error; audit log entry (future). |
| **Extensions** | Owner-only billing actions in cloud mode. |
| **Data** | `role`, `permission` checks |
| **Phase** | P1 |
| **API/UI** | Middleware + UI hide/disable |

### UC-B2: Terminal access gate

| Field | Content |
|--------|---------|
| **Goal** | Only allowed users open browser terminal. |
| **Actors** | TeamMember |
| **Preconditions** | Instance + team terminal policy set. |
| **Main success** | User with `can_access_terminal` opens terminal; others see denial. |
| **Extensions** | Per-server terminal policy overrides. |
| **Data** | `terminal_policy`, membership flags |
| **Phase** | P3 |
| **API/UI** | Terminal WebSocket + policy checks |

---

## Part C — Accounts, authentication, profile

### UC-C1: Register and login (email/password)

| Field | Content |
|--------|---------|
| **Goal** | User accesses the app securely. |
| **Actors** | New user |
| **Preconditions** | Instance allows registration (config). |
| **Main success** | 1) Register email/password. 2) Login. 3) JWT/session issued. 4) Redirect to dashboard or onboarding. |
| **Extensions** | Email verification required; forced password reset; 2FA challenge (future). |
| **Data** | `user`, `password_hash`, sessions |
| **Phase** | P0–P1 |
| **API/UI** | `POST /api/v1/auth/register`, `POST /api/v1/auth/login` |

### UC-C2: Accept team invitation

| Field | Content |
|--------|---------|
| **Goal** | Join team via invite link. |
| **Actors** | Invitee |
| **Preconditions** | Valid token; invite not expired. |
| **Main success** | 1) Open link. 2) Sign in or register. 3) Accept. 4) Membership created with role. |
| **Extensions** | Invite email mismatch; max seats (cloud). |
| **Data** | `invitation`, `team_membership` |
| **Phase** | P1 |
| **API/UI** | Invitation page + API |

---

## Part D — Servers

### UC-D1: Add and validate server (SSH)

| Field | Content |
|--------|---------|
| **Goal** | Team can run workloads on a host. |
| **Actors** | TeamAdmin |
| **Preconditions** | SSH key in team store; host reachable. |
| **Main success** | 1) Enter host, user, port, key. 2) Validate job runs. 3) Docker/proxy checks pass. 4) Server marked ready. |
| **Extensions** | Hetzner provision path; install Docker remotely. |
| **Data** | `server`, `private_key_ref` |
| **Phase** | P2 |
| **API/UI** | Server CRUD, validate endpoint |

### UC-D2: Traefik lifecycle

| Field | Content |
|--------|---------|
| **Goal** | Operator controls reverse proxy on node. |
| **Actors** | TeamAdmin |
| **Preconditions** | Agent/SSH access. |
| **Main success** | Start/stop/restart proxy; view status and logs; edit dynamic rules. |
| **Extensions** | Certificate renewal failures → notifications. |
| **Data** | `server`, proxy state |
| **Phase** | P3 |
| **API/UI** | Proxy actions + log stream |

---

## Part E — Projects and environments

### UC-E1: Environment home — list resources

| Field | Content |
|--------|---------|
| **Goal** | See all apps, DBs, services in one env. |
| **Actors** | TeamMember |
| **Preconditions** | Read access to environment. |
| **Main success** | Dashboard lists resources with status shortcuts. |
| **Extensions** | Filter by tag. |
| **Data** | `application`, `standalone_database`, `service_stack` |
| **Phase** | P1–P3 |
| **API/UI** | Environment detail API + UI |

### UC-E2: New resource wizard (application paths)

| Field | Content |
|--------|---------|
| **Goal** | Create deployable unit from Git, image, or compose. |
| **Actors** | TeamMember (if create allowed) |
| **Preconditions** | Environment selected; source credentials if private. |
| **Main success** | User picks path (public Git, GitHub App, deploy key, Dockerfile, image, compose, empty). 2) Wizard creates `application` row. 3) First deploy queued (future). |
| **Extensions** | Repo webhook registration. |
| **Data** | `application`, git metadata |
| **Phase** | P3 |
| **API/UI** | Create application API + wizard |

---

## Part F — Applications (tabs / areas)

Representative scenarios (each maps to UI sections F.1–F.20):

| ID | Goal | Phase | Notes |
|----|------|-------|--------|
| UC-F1 | Set domains and build commands | P3 | General tab |
| UC-F2 | Manage env vars and secrets | P3 | Masking in API responses |
| UC-F3 | Attach persistent volumes | P3 | Storage tab |
| UC-F4 | Trigger deploy; view deployment log | P3 | Async job + SSE/WebSocket (future) |
| UC-F5 | Rollback to prior deployment | P4 | History + policy |
| UC-F6 | PR preview deployments | P5 | GitHub webhooks |
| UC-F7 | Stream container logs | P3 | Log endpoint |

---

## Part G — Standalone databases

### UC-G1: Create PostgreSQL (example engine)

| Field | Content |
|--------|---------|
| **Goal** | Managed DB resource in environment. |
| **Actors** | TeamAdmin |
| **Preconditions** | Engine enabled; destination available. |
| **Main success** | Create resource; credentials shown once; start container. |
| **Extensions** | Backup schedule; S3 upload. |
| **Data** | `standalone_database`, credentials, `backup_schedule` |
| **Phase** | P4 |
| **API/UI** | DB CRUD + backups |

### UC-G2: Backup now and list executions

| Field | Content |
|--------|---------|
| **Goal** | Operator verifies backup pipeline. |
| **Actors** | TeamAdmin |
| **Preconditions** | Schedule or ad-hoc allowed. |
| **Main success** | Job runs; execution row; notification on failure. |
| **Extensions** | Download from disk vs S3-only message. |
| **Data** | `backup_execution` |
| **Phase** | P4 |
| **API/UI** | Backup API + UI |

---

## Part H — Services (template stacks)

### UC-H1: Deploy stack from catalog

| Field | Content |
|--------|---------|
| **Goal** | One-click multi-container stack. |
| **Actors** | TeamMember |
| **Preconditions** | Template version available; team quota (cloud). |
| **Main success** | Select template; configure domain/env; provision compose. |
| **Extensions** | Per-internal-service overrides. |
| **Data** | `service_stack`, template revision |
| **Phase** | P4 |
| **API/UI** | Service CRUD + catalog sync job |

---

## Parts I–M — Destinations, domains, security keys, storages, shared variables, notifications

| Area | Example scenario | Phase |
|------|------------------|-------|
| I | Create Docker destination on server | P2 |
| I | List domains; attach to app | P3 |
| J | CRUD team SSH keys | P2 |
| J | CRUD cloud tokens; validate | P2 |
| J | Personal API tokens (read/write/deploy) | P1 |
| K | Define S3-compatible storage; link backups | P4 |
| L | Resolve shared vars by scope (team/project/env/server) | P3 |
| M | Configure Discord/Telegram/etc.; receive deploy events | P5 |

---

## Parts N–S — Instance, billing, search, Git sources, files, inbound webhooks

| Area | Example scenario | Phase |
|------|------------------|-------|
| N | Instance SMTP; OAuth providers | P0–P5 |
| O | Stripe subscription sync | P5 |
| P | Global search across team | P3 |
| Q | GitHub App install + repo list | P3 |
| R | Upload backup for restore | P4 |
| S | GitHub push webhook → deploy | P5 |

---

## Part T — Programmatic API

### UC-T1: CLI lists teams with token

| Field | Content |
|--------|---------|
| **Goal** | Automation uses same API as UI. |
| **Actors** | ApiClient |
| **Preconditions** | Valid API token with `read`. |
| **Main success** | `GET /api/v1/teams` returns scoped list. |
| **Extensions** | Masked secrets in JSON. |
| **Data** | `api_token`, permissions |
| **Phase** | P1 |
| **API/UI** | Bearer auth middleware |

---

## Part U — Background behavior (user-visible)

| Scenario | User-visible outcome | Phase |
|----------|----------------------|-------|
| Deploy queued | Status: queued → running → finished/failed | P3 |
| Backup cron | Success/fail + notification | P4 |
| SSL renewal | Certs valid; alert on error | P3 |
| Catalog refresh | New templates appear | P4 |

---

## Phase roadmap (suggested)

1. **P0**: Instance health, public pages, auth shell, demo users (dev).  
2. **P1**: Teams, memberships, projects, environments, API tokens, CLI login/list.  
3. **P2**: Servers, SSH validate, destinations, keys.  
4. **P3**: Applications, deploy pipeline, logs, proxy integration.  
5. **P4+**: Databases, services, backups, notifications, billing, deep integrations.

---

## Document maintenance

When adding an endpoint or page, append a line under **API/UI** for the relevant UC. For new product areas, add a new UC block using the same table template.
