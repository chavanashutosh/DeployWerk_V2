# DeployWerk V2 — enterprise gap analysis and prioritized backlog

**Purpose:** Map the delta between the **enterprise specification** ([docs/spec/](spec/00-overview.md)) and the **current V2 repository** (what runs today). This document is the **authoritative prioritized backlog** by theme. For evidence (files, migrations, implemented vs partial), use [STATUS.md](STATUS.md).

**Last updated:** 2026-06-15.

---

## How to use this document

| Priority | Meaning |
|----------|---------|
| **P0 — Blocker** | Hard to credibly call the product an enterprise platform without this: security, data integrity, or basic operational continuity. |
| **P1 — High** | Major user-facing capability customers expect; absence forces workarounds or external tools. |
| **P2 — Medium** | Meaningful depth; absence degrades but does not necessarily block adoption. |
| **P3 — Roadmap** | Valuable eventually; not a near-term delivery risk. |

**Traceability:** Each area references the relevant spec chapter(s). **Mail platform** backlog is specified in [spec/08-mail-platform.md](spec/08-mail-platform.md) and rolled into maturity waves in [STATUS.md](STATUS.md).

**Schema notes (V2 snapshot):**

- **`api_tokens`** includes `expires_at` (nullable) and `allowed_cidrs` (JSONB CIDR list, nullable). Enforcement: expiry + IP check in [`auth.rs`](../crates/deploywerk-api/src/auth.rs) (`peer_ip_from_headers` for `X-Forwarded-For` / `X-Real-IP`).
- **OTLP** persists raw batches in `otlp_trace_batches` with list/get and retention ([`team_platform.rs`](../crates/deploywerk-api/src/team_platform.rs)); a **trace explorer** and metrics stack remain gaps (see observability section).
- **`team_secret_versions`** and **`team_audit_log`** exist; coverage of all mutating routes and secret-read auditing is still incomplete.

---

## 1. Identity and access

**Spec:** [01-actors-identity-orgs.md](spec/01-actors-identity-orgs.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| SAML 2.0 IdP integration (production-grade) | P0 | Experimental ACS/SP exists behind flags ([`saml.rs`](../crates/deploywerk-api/src/saml.rs)); needs signature verification, robust metadata, and operator hardening before enterprise claims. |
| MFA enforcement at org level | P1 | Org `mfa_required` + TOTP for **local password** logins ([`mfa.rs`](../crates/deploywerk-api/src/mfa.rs), [`handlers.rs`](../crates/deploywerk-api/src/handlers.rs)); OIDC-primary orgs may still bypass product MFA — clarify policy or add IdP-step-up. |
| Session revocation UI | P1 | API tokens can be revoked; no UI to list or terminate active JWT sessions. |
| Passkey / WebAuthn primary auth | P1 | Modern baseline; reduces phishing for password-based paths. |
| Step-up authentication for sensitive actions | P1 | High-risk actions (e.g. delete team, reveal secret, prod deploy approve) should re-verify; no gate today. |
| IP-allowlist enforcement per token | **Addressed (MVP)** | `allowed_cidrs` JSON + enforcement; operators must trust proxy headers. |
| Token expiry enforcement | **Addressed (MVP)** | `expires_at` + auth rejection in [`auth.rs`](../crates/deploywerk-api/src/auth.rs). |
| JIT privilege with auto-expiry | P2 | Time-bounded role grants (incident, contractor). |
| Custom RBAC roles | P2 | Owner/admin/member only; spec describes permission catalog + user-defined compositions. |
| Attribute-based access control (ABAC) | P2 | Tags, tier, MFA-state conditions. |
| OPA/Rego policy engine | P3 | Policy-as-code on mutating operations; high effort; best after ABAC primitives. |

---

## 2. Secrets management

**Spec:** [04-security-rbac-secrets-compliance.md](spec/04-security-rbac-secrets-compliance.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| Secret versioning (full spec) | P1 | Version history + `dw_secret:NAME@VERSION` ([`team_secrets.rs`](../crates/deploywerk-api/src/team_secrets.rs)); rotation policies/alerts still missing. |
| KMS integration (Vault, AWS KMS, Azure, GCP) | P1 | Secrets encrypted with operator key; no external KMS wrapping / recovery story for lost instance keys. |
| Secret rotation policies | P1 | No scheduled or event-triggered rotation; no “not rotated in N days” alert. |
| Secret access audit log | P1 | Plaintext reads should emit audit records; not confirmed for `team_secrets` paths. |
| ACME-driven TLS certificate auto-rotation | P1 | Certificates often manual; spec ties ACME to rotation and re-deploy. |
| Workload identity tokens | P2 | Short-lived container-scoped credentials; needs agent v2. |
| SPIFFE/SPIRE integration | P3 | SVID-based mTLS; prerequisite: workload identity. |

---

## 3. Deploy lifecycle

**Spec:** [03-deploy-lifecycle-registry.md](spec/03-deploy-lifecycle-registry.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| Blue/green traffic shifting | P1 | `deploy_strategy` exists; atomic cutover / keeping old version warm not confirmed. |
| Canary deployments | P1 | Weighted routing, promote/abort on metrics; same scaffold gap. |
| Pre-deploy and post-deploy hooks | **Partial** | HTTP POST hooks on application; worker enforces success ([`applications.rs`](../crates/deploywerk-api/src/applications.rs)); no arbitrary shell stages yet. |
| Deploy approval notifications | **Addressed (MVP)** | `deploy_pending_approval` notifications when job enters pending approval ([`applications.rs`](../crates/deploywerk-api/src/applications.rs) `enqueue_deploy_inner`). |
| Rolling deploy across server groups | P2 | Max-unavailable across destinations. |
| SLO-gated deploys | P2 | Block when error budget exhausted; needs SLO engine. |
| Parallel deploy strategies | P2 | Fan-out / fan-in across applications. |

---

## 4. Container registry

**Spec:** [03-deploy-lifecycle-registry.md](spec/03-deploy-lifecycle-registry.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| OCI-compliant registry (first-class) | P1 | `registry_status` stub (`integrated: false`); teams rely on external registries. |
| CVE vulnerability scanning on push | P1 | No scanner; cannot enforce “block Critical CVE” at gate. |
| Image signing and attestation | P2 | Sigstore-style provenance before deploy. |
| Image replication | P2 | Mirror for DR / multi-site. |
| Immutable tags and GC policy | P2 | No overwrite; scheduled layer GC. |

---

## 5. Observability

**Spec:** [05-platform-networking-iac-observability.md](spec/05-platform-networking-iac-observability.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| OTLP trace storage and explorer | P1 | Raw batch storage + list/download; **no** trace explorer or parsing — either deepen or integrate external backend. |
| Metrics ingest and time-series storage | P1 | No TSDB; health check rows are not metrics. |
| Log platform (structured ingest, search) | P1 | Job logs streamed/stored; runtime logs not persistently indexed; no log explorer. |
| Custom dashboard builder | P2 | Needs metrics storage. |
| SLO management engine | P2 | Error budget, burn alerts; needs metrics. |
| Trace-to-log correlation | P2 | Needs traces + log platform. |
| Alert manager (grouping, routing, dedup) | P2 | Notification endpoints exist; no evaluation engine above health transitions. |
| On-call rotation schedules | P3 | Pager / rotations. |

---

## 6. Compliance and audit

**Spec:** [04-security-rbac-secrets-compliance.md](spec/04-security-rbac-secrets-compliance.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| Audit log completeness | P1 | `team_audit_log` + partial instrumentation ([`audit.rs`](../crates/deploywerk-api/src/audit.rs)); expand to all sensitive routes + exports. |
| Tamper-evident audit storage (hash chaining) | P1 | `chain_hint` migration exists; cryptographic completeness unconfirmed in API. |
| SIEM streaming (Splunk, Elastic, S3) | P1 | No outbound audit stream; admin API only. |
| Compliance report templates | P2 | SOC 2, ISO 27001, HIPAA, PCI evidence packs. |
| Evidence collector (point-in-time snapshots) | P2 | Signed snapshots for auditors. |
| GDPR erasure workflow | P2 | Deactivate, redact PII, cold archive. |
| Data residency controls | P3 | Region-bound storage per tenant. |

---

## 7. Infrastructure and networking

**Spec:** [05-platform-networking-iac-observability.md](spec/05-platform-networking-iac-observability.md), [07-operations-enterprise.md](spec/07-operations-enterprise.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| Agent v2: deploy execution + metrics + log shipping | P1 | Agent is heartbeat-only; spec requires execution, metrics, logs, policy, air-gap. |
| Edge / reverse proxy layer | P1 | Firewall/CDN stored + Traefik snippets; no Rust-native edge as in spec. |
| mTLS between services | P2 | Needs edge / identity story. |
| Service discovery | P2 | Logical names without hardcoded IPs. |
| Private overlay networks per team | P2 | Isolation on shared infra. |
| Rate limiting at proxy layer | P2 | Per IP/token/header. |
| Air-gap agent mode | P3 | Local queue, sync on reconnect. |

---

## 8. Developer experience

**Spec:** [06-developer-experience-and-data.md](spec/06-developer-experience-and-data.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| Service catalog and developer portal | P1 | Primary self-service discovery; absent. |
| Self-service provisioning forms | P1 | Non-admins cannot provision projects/environments without admin. |
| Golden path templates | P1 | Starter templates for common app types. |
| Visual pipeline builder | P2 | No-YAML canvas; Git auto-deploy is closest. |
| No-code site builder | P2 | Drag-and-drop authoring + deploy. |
| Production-readiness scorecards | P2 | Automated checks (SLO, runbook, backup, scan). |
| API documentation hosting | P3 | OpenAPI renderer per service. |

---

## 9. Cost and resource governance

**Spec:** [05-platform-networking-iac-observability.md](spec/05-platform-networking-iac-observability.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| CPU and memory quota enforcement at deploy time | P1 | Limits on applications; no team quota enforcement blocking deploy. |
| Cost model and showback | P2 | Synthetic $/CPU/GB-hour. |
| Chargeback export | P2 | CSV/API by cost center. |
| Budget alerts | P2 | Monthly synthetic spend thresholds. |
| Right-sizing recommendations | P3 | Limits vs observed peak. |

---

## 10. GitOps and change management

**Spec:** [05-platform-networking-iac-observability.md](spec/05-platform-networking-iac-observability.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| IaC providers (Terraform, Pulumi) | P2 | No provider; API-only GitOps. |
| GitOps reconciliation loop | P2 | Full config from Git vs deploy-only hooks today. |
| Drift detection | P2 | Live vs last-applied. |
| Change request (CR) workflow | P2 | Risk, rollback, multi-approver beyond simple deploy approval. |
| Change freeze calendar | P2 | Org-wide freeze; `deploy_schedule` is per-environment building block. |
| CAB quorum | P3 | Named approver group + quorum. |

---

## 11. Disaster recovery

**Spec:** [05-platform-networking-iac-observability.md](spec/05-platform-networking-iac-observability.md), [07-operations-enterprise.md](spec/07-operations-enterprise.md)

| Gap | Priority | Notes |
|-----|----------|-------|
| Configuration snapshot and restore | P1 | No backup of app/env/secret config for team recovery. |
| Database backup hooks | P2 | Pre-deploy backup container + retention. |
| Cross-site replication | P3 | Secondary DeployWerk instance failover. |

---

## 12. Mail platform (cross-reference)

Capabilities, phases, and schema intent: **[spec/08-mail-platform.md](spec/08-mail-platform.md)**. Mail is tracked as a **major product vertical** alongside the gaps above; delivery is phased (core → reliability → enterprise) and aligned with maturity waves in [STATUS.md](STATUS.md).

---

## Quick links

- [STATUS.md](STATUS.md) — implementation evidence and technical inventory
- [spec/00-overview.md](spec/00-overview.md) — intended product surface
- [spec/08-mail-platform.md](spec/08-mail-platform.md) — mail platform specification
