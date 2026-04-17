const TOKEN_KEY = "deploywerk_token";

function stripTrailingSlash(s: string): string {
  return s.replace(/\/+$/, "");
}

/**
 * When set at build time (`VITE_API_URL`), all API calls use this origin.
 * When empty, requests use relative `/api/...` (Vite dev/preview proxy or same-origin reverse proxy).
 */
export function apiOrigin(): string {
  const raw = import.meta.env.VITE_API_URL?.trim();
  if (!raw) return "";
  return stripTrailingSlash(raw);
}

export function resolveApiUrl(path: string): string {
  if (path.startsWith("http://") || path.startsWith("https://")) return path;
  const o = apiOrigin();
  if (!o) return path;
  const p = path.startsWith("/") ? path : `/${path}`;
  return `${o}${p}`;
}

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string | null) {
  if (token) localStorage.setItem(TOKEN_KEY, token);
  else localStorage.removeItem(TOKEN_KEY);
}

export async function apiFetchRaw(path: string, init?: RequestInit): Promise<Response> {
  const headers = new Headers(init?.headers);
  const t = getToken();
  if (t) headers.set("Authorization", `Bearer ${t}`);
  return fetch(resolveApiUrl(path), { ...init, headers });
}

export async function apiFetch<T>(
  path: string,
  init?: RequestInit,
): Promise<T> {
  const headers = new Headers(init?.headers);
  if (init?.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  const t = getToken();
  if (t) headers.set("Authorization", `Bearer ${t}`);

  const res = await fetch(resolveApiUrl(path), { ...init, headers });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || res.statusText);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

export async function putCurrentTeam(teamId: string): Promise<void> {
  await apiFetch("/api/v1/me/current-team", {
    method: "PUT",
    body: JSON.stringify({ team_id: teamId }),
  });
}

export async function putCurrentOrganization(organizationId: string): Promise<void> {
  await apiFetch("/api/v1/me/current-organization", {
    method: "PUT",
    body: JSON.stringify({ organization_id: organizationId }),
  });
}

export type Organization = {
  id: string;
  name: string;
  slug: string;
  role: "owner" | "admin" | "member";
  mfa_required?: boolean;
};

export type ApplicationMembership = {
  application_id: string;
  role: "admin" | "viewer";
};

export type User = {
  id: string;
  email: string;
  name: string | null;
  current_team_id?: string | null;
  current_organization_id?: string | null;
  settings?: Record<string, unknown> | null;
  /** Instance super admin (admin console); not team/org role. */
  is_platform_admin?: boolean;
  /** Orgs where user is owner or admin (org settings, not app-admin). */
  organization_admin_organization_ids?: string[];
  application_memberships?: ApplicationMembership[];
};

export async function patchMe(body: {
  name?: string;
  current_password?: string;
  new_password?: string;
  settings?: Record<string, unknown>;
}): Promise<User> {
  return apiFetch<User>("/api/v1/me", {
    method: "PATCH",
    body: JSON.stringify(body),
  });
}

export type Team = {
  id: string;
  organization_id: string;
  name: string;
  slug: string;
  role: "owner" | "admin" | "member";
  /** Reached via org owner/admin, not team membership. */
  access_via_organization_admin?: boolean;
};

/** Metadata from GET /teams/{id}/secrets (values are never returned). */
export type TeamSecretMeta = {
  name: string;
  updated_at: string;
};

/** GET /teams/{id}/registry/status */
export type RegistryStatusResponse = {
  integrated: boolean;
  team_id: string;
  hint: string;
};

/** GET /teams/{id}/cost/summary */
export type CostSummaryResponse = {
  team_id: string;
  currency: string;
  synthetic_monthly_estimate: unknown | null;
  note: string;
};

export type TeamInvitationRow = {
  id: string;
  email: string;
  role: "owner" | "admin" | "member";
  expires_at: string;
  accepted: boolean;
};

export type Project = {
  id: string;
  team_id: string;
  name: string;
  slug: string;
  description?: string | null;
  created_at: string;
};

export type Environment = {
  id: string;
  project_id: string;
  name: string;
  slug: string;
  created_at: string;
  deploy_locked?: boolean;
  deploy_lock_reason?: string | null;
  deploy_schedule_json?: string | null;
};

export type Server = {
  id: string;
  team_id: string;
  name: string;
  host: string;
  ssh_port: number;
  ssh_user: string;
  status: "pending" | "ready" | "error";
  last_validated_at?: string | null;
  last_validation_error?: string | null;
  created_at: string;
};

export type ValidateServerResponse = {
  ok: boolean;
  detail?: string;
};

/** Instance operator links from `GET /api/v1/bootstrap` (`DEPLOYWERK_INTEGRATION_*`). */
export type PlatformIntegrationsBootstrap = {
  /** True when API applied DEPLOYWERK_LOCAL_SERVICE_DEFAULTS (127.0.0.1 preset). */
  localServiceDefaults?: boolean;
  forgejoUrl?: string | null;
  mailcowUrl?: string | null;
  portainerUrl?: string | null;
  technitiumUrl?: string | null;
  matrixClientUrl?: string | null;
  traefikDashboardUrl?: string | null;
  /** In-app route or external docs. */
  ssoPlaybookUrl?: string | null;
  technitiumDnsAutomationConfigured?: boolean;
  portainerHealthProbeConfigured?: boolean;
};

export type Bootstrap = {
  demo_logins_enabled: boolean;
  allow_local_password_auth: boolean;
  oidc_enabled: boolean;
  /** Built-in "Platform (API host)" destination when true. */
  platform_docker_enabled?: boolean;
  /** Base domain for auto-generated app hostnames (wildcard DNS at the edge). */
  apps_base_domain?: string | null;
  /** OIDC issuer URL when SSO is configured (e.g. Authentik application issuer). */
  authentik_issuer?: string | null;
  /** Operator link to Authentik admin UI when configured. */
  idp_admin_url?: string | null;
  demo_accounts?: { email: string; role: string; password: string }[];
  /** `DEPLOYWERK_SMTP_HOST` + `DEPLOYWERK_SMTP_FROM` set on the API. */
  mail_smtp_configured?: boolean;
  /** `DEPLOYWERK_PUBLIC_APP_URL` set (invite links in email). */
  public_app_url_configured?: boolean;
  /** Public app origin when configured (webhook URL hints). */
  public_app_url?: string | null;
  platform_integrations?: PlatformIntegrationsBootstrap;
};

export type OidcConfig = {
  enabled: boolean;
  issuer?: string;
  client_id?: string;
  redirect_uri?: string | null;
  authorization_endpoint?: string;
  token_endpoint?: string;
  scopes?: string;
};

export type InvitationPublic = {
  team_name: string;
  team_slug: string;
  email: string;
  role: "owner" | "admin" | "member";
  expires_at: string;
  accepted: boolean;
  expired?: boolean;
};

export type Destination = {
  id: string;
  team_id: string;
  /** Absent for `docker_platform` (API host). */
  server_id?: string | null;
  name: string;
  slug: string;
  kind: "docker_standalone" | "docker_platform";
  description?: string | null;
  created_at: string;
};

export type Application = {
  id: string;
  environment_id: string;
  destination_id?: string | null;
  name: string;
  slug: string;
  docker_image: string;
  /** Resolved image from the last successful standard/rollback deploy (server-side). */
  last_deployed_image?: string | null;
  /** Prior successful image; rollback redeploys this. */
  previous_deployed_image?: string | null;
  domains: string[];
  /** Stable hostname under `apps_base_domain` for edge routing (Traefik Host rule). */
  auto_hostname?: string | null;
  git_repo_url?: string | null;
  /** Normalized owner/repo for GitHub webhooks */
  git_repo_full_name?: string | null;
  auto_deploy_on_push?: boolean;
  /** Exact branch, `*`, or `prefix/*` */
  git_branch_pattern?: string;
  build_image_from_git?: boolean;
  git_build_ref?: string;
  dockerfile_path?: string;
  /** GitHub App pull_request preview deploys when webhook is configured. */
  pr_preview_enabled?: boolean;
  created_at: string;
  deploy_strategy?: string;
  require_deploy_approval?: boolean;
  pre_deploy_hook_url?: string | null;
  post_deploy_hook_url?: string | null;
};

export type ApplicationEnvVarPublic = {
  key: string;
  value?: string | null;
  is_secret: boolean;
};

/** Host path is derived on the worker from `name` under `DEPLOYWERK_VOLUMES_ROOT`. */
export type RuntimeVolumeMount = {
  name: string;
  container_path: string;
};

export type ApplicationDetail = Application & {
  env_vars: ApplicationEnvVarPublic[];
  runtime_volumes?: RuntimeVolumeMount[];
};

export type DeployJobStatus =
  | "pending_approval"
  | "queued"
  | "running"
  | "succeeded"
  | "failed";

export type DeployJob = {
  id: string;
  application_id: string;
  status: DeployJobStatus;
  log: string;
  created_at: string;
  started_at?: string | null;
  finished_at?: string | null;
  git_ref?: string | null;
  git_sha?: string | null;
  job_kind?: string | null;
  deploy_strategy?: string | null;
  approved_at?: string | null;
  /** Object storage key for full deploy log (when configured). */
  log_object_key?: string | null;
  /** Object storage key for deploy manifest JSON (when configured). */
  artifact_manifest_key?: string | null;
};

export type EnqueueDeployResponse = {
  job_id: string;
  status: DeployJobStatus;
};

export type TeamDeploymentRow = {
  job_id: string;
  application_id: string;
  application_name: string;
  application_slug: string;
  environment_id: string;
  environment_name: string;
  project_id: string;
  project_name: string;
  status: DeployJobStatus;
  created_at: string;
  started_at?: string | null;
  finished_at?: string | null;
  git_ref?: string | null;
  git_sha?: string | null;
  /** When set, job is not a normal production deploy (e.g. PR preview). */
  job_kind?: string | null;
  pr_number?: number | null;
  git_repo_full_name?: string | null;
  /** HTTPS URL for the app (auto hostname or first domain). */
  primary_url?: string | null;
  /** GitHub / GitLab commit page when derivable from repo + sha. */
  source_commit_url?: string | null;
  git_base_sha?: string | null;
  /** GitHub / GitLab compare base...head when PR preview stored base + head. */
  source_compare_url?: string | null;
  /** Effective strategy for this job (application default if unset on job). */
  deploy_strategy?: string;
};

export type TeamDomainRow = {
  domain: string;
  application_id: string;
  application_name: string;
  environment_name: string;
  project_name: string;
  /** Matches API-provisioned `auto_hostname`. */
  provisioned?: boolean;
};
