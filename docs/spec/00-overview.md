# Overview

## Architecture note

DeployWerk is built in **Rust end-to-end**. The core daemon, API server, job runner, log streamer, agent protocol, CLI, and all background workers are Rust binaries. This gives the platform predictable latency, low memory footprint per tenant, safe concurrency for deploy orchestration, and a single statically linked binary deployment model for the operator. All performance and reliability claims in this specification are grounded in that foundation.

## What DeployWerk is

DeployWerk is a **self-hosted, self-service application delivery platform** for engineering organizations of any size. Teams bring their own infrastructure — bare metal, VPS, private cloud, or hybrid — and DeployWerk handles the full lifecycle: container orchestration, secrets, routing, observability, compliance, cost governance, and developer self-service, without handing control to a managed cloud vendor.

## Five capability zones

| Zone | Purpose |
|------|---------|
| **Core deploy lifecycle** | Projects, environments, servers, destinations, applications, deploy jobs, rollback, logs |
| **Enterprise operations** | RBAC, audit, compliance, secret management, policy engine, DR, cost governance |
| **Developer self-service** | Service catalog, developer portal, visual pipeline builder, no-code site builder |
| **Observability and reliability** | Metrics, tracing, logging, SLO/SLA, health checks, incident management |
| **Platform administration** | Multi-tenant oversight, billing, entitlements, identity federation |

## Summary matrix

| Area | Expectation |
|------|-------------|
| Auth, orgs, teams, invites, projects, environments | **Core** — fully realized collaboration structure |
| Servers, destinations, applications, deploy jobs, logs, rollback | **Core** — full deploy lifecycle |
| Blue/green, canary, rolling deploy strategies | **Core** — production-grade deploy control |
| Container registry with scanning and signing | **Core** — first-class image management |
| Secret management, workload identity, KMS integration | **Core** — enterprise secret hygiene |
| Advanced RBAC, custom roles, policy engine (OPA/Rego) | **Core** — fine-grained enterprise authorization |
| Audit log, compliance reports, evidence collector | **Core** — governance and audit readiness |
| Infrastructure as Code (Terraform, Pulumi providers) | **Core** — GitOps-compatible |
| Networking, mTLS, service discovery, traffic shaping | **Core** — production network control |
| Observability (metrics, traces, logs, alerting) | **Core** — integrated; no mandatory external APM |
| SLO/SLA management, error budgets, SLO-gated deploys | **Core** — reliability engineering built in |
| Cost management, quotas, chargeback | **Core** — FinOps-ready |
| Disaster recovery, backup, cross-site replication | **Core** — enterprise continuity |
| GitOps, change management, change freeze, CAB | **Core** — enterprise change control |
| Developer portal, service catalog, golden paths | **Core** — internal developer platform layer |
| Visual pipeline builder (no-code CI/CD) | **Core** — self-service pipelines |
| No-code website and application builder | **Core** — full drag-and-drop site authoring and publishing |
| Database management with migration runner | **Core** — integrated data tier |
| Marketplace and extensions | **Core** — extensible plugin architecture |
| Enterprise billing, entitlements, chargeback | **Core** when billing integration configured |
| Git push hooks, GitHub App, PR previews | **Partial** — powerful when configured; not full vendor parity |
| OIDC sign-in, SAML 2.0, SCIM provisioning | **Optional** — only when instance configured |
| Workload identity, SPIFFE/SPIRE | **Optional** — when identity broker configured |
| Flags, storage, firewall, CDN, RUM, AI gateway, sandboxes | **Extended / variable depth** — evolving surface |
| Team-managed mail, transactional send API (Stalwart) | **Extended** — self-hosted mail aligned with domains, secrets, deploy; see [08-mail-platform.md](08-mail-platform.md) |
| Platform admin (multi-tenant oversight) | **Core** for multi-tenant operator |

## Rust architecture notes

All performance and behavioral properties described in this specification are grounded in the Rust foundation:

- **Deploy orchestration engine:** async Tokio runtime; hundreds of concurrent deploy jobs with minimal per-job overhead.
- **Log streamer:** broadcast channel fan-out; multiple subscribers (browser tabs, CLI `--watch`, webhook delivery) receive the same log stream from a single read path.
- **Agent protocol:** async WebSocket with binary framing; agents maintain persistent low-overhead connections to the control plane.
- **Proxy / edge layer:** Hyper-based reverse proxy with zero-copy body forwarding; TLS termination via rustls (no OpenSSL dependency).
- **Secret encryption:** `ring` crate for AES-256-GCM; no runtime FFI to C crypto libraries.
- **Metrics storage:** custom columnar store with LZ4 compression; lower memory footprint than a general-purpose TSDB for typical team data volumes.
- **WASM extension sandbox:** Wasmtime runtime for extension isolation; extensions cannot access host memory outside the declared interface.
- **Static binary distribution:** operator deploys a single binary per component; no runtime dependencies on the host OS beyond libc.
- **No-code builder renderer:** site preview served by a lightweight Axum handler; build output is a static bundle served efficiently from the edge layer.

**See also:** [01-actors-identity-orgs.md](01-actors-identity-orgs.md) through [07-operations-enterprise.md](07-operations-enterprise.md) for full functional detail; [08-mail-platform.md](08-mail-platform.md) for team mail and transactional delivery.
