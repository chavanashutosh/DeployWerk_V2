# DeployWerk documentation

This folder holds the **Enterprise Platform Specification**: the intended product surface for DeployWerk as a self-hosted, enterprise-grade application delivery platform. It describes **what the product is meant to provide**, not a line-by-line map of the current repository.

**Implementation** (API routes, migrations, UI wiring) remains the source of truth in code (`crates/`, `web/`). Use your editor and `rg` to explore behavior.

## Specification index

| Document | Contents |
|----------|----------|
| [spec/00-overview.md](spec/00-overview.md) | Rust architecture note; what DeployWerk is; five capability zones; summary matrix; Rust implementation notes |
| [spec/01-actors-identity-orgs.md](spec/01-actors-identity-orgs.md) | Actors; identity, session, workspace; organizations and teams |
| [spec/02-projects-through-applications.md](spec/02-projects-through-applications.md) | Projects and environments; servers and destinations; applications |
| [spec/03-deploy-lifecycle-registry.md](spec/03-deploy-lifecycle-registry.md) | Deployments, jobs, rollback, logs; container registry |
| [spec/04-security-rbac-secrets-compliance.md](spec/04-security-rbac-secrets-compliance.md) | Secret management; advanced RBAC and policy; audit and compliance; secret-zero and workload identity |
| [spec/05-platform-networking-iac-observability.md](spec/05-platform-networking-iac-observability.md) | Infrastructure as code; networking and routing; observability; SLO/SLA; cost; DR/backup; GitOps and change management |
| [spec/06-developer-experience-and-data.md](spec/06-developer-experience-and-data.md) | Developer portal; visual pipeline builder; no-code site builder; database management; marketplace and extensions |
| [spec/07-operations-enterprise.md](spec/07-operations-enterprise.md) | Enterprise billing; notifications; health checks; API tokens and CLI; agent and edge; platform administration; public and legal pages |
| [spec/08-mail-platform.md](spec/08-mail-platform.md) | Team-managed mail (Stalwart), domains/DKIM, webmail (JMAP), transactional mail API — intended product surface |

## Operator and admin notes

- [BARE_METAL.md](BARE_METAL.md) — **bare metal**: Ubuntu 24.04, DeployWerk (Host A), optional Mailcow/Matrix/Technitium (Host B), Let’s Encrypt, XRDP.
- [STATUS.md](STATUS.md) — **spec vs implementation**: what is done, what is partial/placeholder, and what is pending (single consolidated status doc).
- [ENTERPRISE_GAPS.md](ENTERPRISE_GAPS.md) — **prioritized gap backlog** (P0–P3) vs the enterprise spec; links to evidence in STATUS.
- [LOGICAL_CAPABILITY_MAP.md](LOGICAL_CAPABILITY_MAP.md) — thin index from capability areas to primary Rust modules / UI entry points.
- [ADMIN_DEV.md](ADMIN_DEV.md) — local development pointers for platform admin routes and API checks.
