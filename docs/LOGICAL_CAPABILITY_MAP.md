# Logical capability map (thin index)

**Purpose:** Fast pointers from a **capability area** to the primary **Rust modules**, **web routes**, or **migrations** in this repository. It is not exhaustive; authoritative detail remains in [STATUS.md](STATUS.md) and code.

**Last updated:** 2026-06-15.

| Area | Primary API / Rust | Web / UI | Notable migrations |
|------|-------------------|----------|-------------------|
| HTTP router composition | [lib.rs](../crates/deploywerk-api/src/lib.rs) | — | — |
| Auth, JWT, API token validation | [auth.rs](../crates/deploywerk-api/src/auth.rs) | [auth.tsx](../web/src/auth.tsx), [LoginPage.tsx](../web/src/pages/LoginPage.tsx), [TokensPage.tsx](../web/src/pages/TokensPage.tsx) | `api_tokens` + [20260614123000_api_tokens_expiry.sql](../crates/deploywerk-api/migrations/20260614123000_api_tokens_expiry.sql), [20260615120000_api_tokens_allowed_cidrs.sql](../crates/deploywerk-api/migrations/20260615120000_api_tokens_allowed_cidrs.sql) |
| Core REST (teams, projects, tokens, …) | [handlers.rs](../crates/deploywerk-api/src/handlers.rs) | [App.tsx](../web/src/App.tsx), [lazyPages.ts](../web/src/lazyPages.ts) | initial postgres |
| Organizations | [organizations.rs](../crates/deploywerk-api/src/organizations.rs) | settings pages under `web/src/pages/app/settings/` | [20260423120000_organizations.sql](../crates/deploywerk-api/migrations/20260423120000_organizations.sql) |
| Applications, deploy jobs | [applications.rs](../crates/deploywerk-api/src/applications.rs) | [ApplicationsPage.tsx](../web/src/pages/ApplicationsPage.tsx), [DeploymentsPage.tsx](../web/src/pages/app/DeploymentsPage.tsx) | [20260412130000_applications_deploy_jobs.sql](../crates/deploywerk-api/migrations/20260412130000_applications_deploy_jobs.sql), control plane |
| Servers, remote Docker | [servers.rs](../crates/deploywerk-api/src/servers.rs) | [ServersPage.tsx](../web/src/pages/ServersPage.tsx), [ServerDockerPage.tsx](../web/src/pages/ServerDockerPage.tsx) | [20260411130000_servers.sql](../crates/deploywerk-api/migrations/20260411130000_servers.sql) |
| Destinations | [destinations.rs](../crates/deploywerk-api/src/destinations.rs) | [DestinationsPage.tsx](../web/src/pages/DestinationsPage.tsx) | [20260412120000_destinations.sql](../crates/deploywerk-api/migrations/20260412120000_destinations.sql) |
| Team secrets | [team_secrets.rs](../crates/deploywerk-api/src/team_secrets.rs) | [TeamSecretsSettingsPage.tsx](../web/src/pages/app/settings/TeamSecretsSettingsPage.tsx) | [20260502120000_team_secrets.sql](../crates/deploywerk-api/migrations/20260502120000_team_secrets.sql), [20260614121000_team_secret_versions.sql](../crates/deploywerk-api/migrations/20260614121000_team_secret_versions.sql) |
| Notifications | [notifications.rs](../crates/deploywerk-api/src/notifications.rs) | [NotificationEndpointsPanel.tsx](../web/src/components/team/NotificationEndpointsPanel.tsx) | notification kinds migration |
| Transactional SMTP (instance) | [mail.rs](../crates/deploywerk-api/src/mail.rs) | — | — |
| Platform team APIs (OTLP batches, registry stub, …) | [team_platform.rs](../crates/deploywerk-api/src/team_platform.rs) | [placeholders.tsx](../web/src/pages/app/placeholders.tsx), [platform/](../web/src/pages/app/platform/) | placeholders + [20260614120000_otlp_trace_batches.sql](../crates/deploywerk-api/migrations/20260614120000_otlp_trace_batches.sql) |
| Team audit log | [audit.rs](../crates/deploywerk-api/src/audit.rs), [team_platform.rs](../crates/deploywerk-api/src/team_platform.rs) (`/audit-log`) | [TeamAuditSettingsPage.tsx](../web/src/pages/app/settings/TeamAuditSettingsPage.tsx) | [20260614122000_team_audit_log.sql](../crates/deploywerk-api/migrations/20260614122000_team_audit_log.sql) |
| MFA / experimental SAML | [mfa.rs](../crates/deploywerk-api/src/mfa.rs), [saml.rs](../crates/deploywerk-api/src/saml.rs) | [LoginPage.tsx](../web/src/pages/LoginPage.tsx), [OrganizationSettingsPage.tsx](../web/src/pages/app/settings/OrganizationSettingsPage.tsx) | [20260614124000_org_mfa_and_saml.sql](../crates/deploywerk-api/migrations/20260614124000_org_mfa_and_saml.sql) |
| RBAC | [rbac.rs](../crates/deploywerk-api/src/rbac.rs), [permissions_catalog.rs](../crates/deploywerk-api/src/permissions_catalog.rs) | — | app memberships |
| OIDC | [oidc.rs](../crates/deploywerk-api/src/oidc.rs) | [OidcCallbackPage.tsx](../web/src/pages/OidcCallbackPage.tsx) | — |
| SCIM | [scim.rs](../crates/deploywerk-api/src/scim.rs) | — | [20260426120000_authentik_scim.sql](../crates/deploywerk-api/migrations/20260426120000_authentik_scim.sql) |
| Platform admin | [admin.rs](../crates/deploywerk-api/src/admin.rs) | [admin/](../web/src/pages/admin/) | [20260425120000_platform_admin.sql](../crates/deploywerk-api/migrations/20260425120000_platform_admin.sql) |
| GitHub webhooks | [webhook_github.rs](../crates/deploywerk-api/src/webhook_github.rs), [github_app_api.rs](../crates/deploywerk-api/src/github_app_api.rs) | — | git / PR preview migrations |
| Deploy worker (binary) | [deploywerk_deploy_worker.rs](../crates/deploywerk-api/src/bin/deploywerk_deploy_worker.rs) | — | — |
| Host agent | [deploywerk-agent/src/main.rs](../crates/deploywerk-agent/src/main.rs) | — | — |
| CLI | [deploywerk-cli/src/main.rs](../crates/deploywerk-cli/src/main.rs) | — | — |
| Mail platform (Phase 1 slice) | [mail_platform.rs](../crates/deploywerk-api/src/mail_platform.rs) | [MailDomainsSettingsPage.tsx](../web/src/pages/app/settings/MailDomainsSettingsPage.tsx) | [20260614125000_mail_platform_phase1.sql](../crates/deploywerk-api/migrations/20260614125000_mail_platform_phase1.sql) |

---

**Maintaining:** When adding a major module or migration, add or adjust **one row** here and expand narrative in [STATUS.md](STATUS.md).
