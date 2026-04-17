import { ArrowLeft, Eye, ShieldCheck, Terminal } from "lucide-react";
import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { apiFetch, apiOrigin } from "@/api";
import { toastError, toastSuccess } from "@/toast";
import { AdminViewLink, formatAdminListError } from "./adminUi";

function cliSuggestedApiBase(): string {
  const o = apiOrigin().trim();
  if (o) return o;
  return typeof window !== "undefined" ? window.location.origin : "http://localhost:8080";
}

type Detail = {
  user: {
    id: string;
    email: string;
    name: string | null;
    created_at: string;
    is_platform_admin: boolean;
  };
  organization_memberships: {
    organization_id: string;
    org_name: string;
    org_slug: string;
    role: string;
  }[];
  team_memberships: {
    team_id: string;
    team_name: string;
    team_slug: string;
    organization_id: string;
    role: string;
  }[];
};

export function AdminUserDetailPage() {
  const { id = "" } = useParams();
  const [d, setD] = useState<Detail | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function load() {
    if (!id) return;
    try {
      const x = await apiFetch<Detail>(`/api/v1/admin/users/${id}`);
      setD(x);
      setErr(null);
    } catch (e) {
      setErr(formatAdminListError(e));
    }
  }

  useEffect(() => {
    void load();
  }, [id]);

  async function togglePlatformAdmin(next: boolean) {
    if (!id) return;
    setSaving(true);
    try {
      await apiFetch(`/api/v1/admin/users/${id}/platform-admin`, {
        method: "PATCH",
        body: JSON.stringify({ is_platform_admin: next }),
      });
      toastSuccess(next ? "Super admin access granted" : "Super admin access revoked");
      await load();
    } catch (e) {
      const m = formatAdminListError(e);
      setErr(m);
      toastError(m);
    } finally {
      setSaving(false);
    }
  }

  if (!d && !err) {
    return <p className="text-slate-600">Loading…</p>;
  }
  if (err && !d) {
    return (
      <p className="text-red-600">
        {err}{" "}
        <Link to="/admin/users" className="inline-flex items-center gap-1 text-violet-700 hover:underline">
          <ArrowLeft className="h-4 w-4" strokeWidth={1.75} aria-hidden />
          Users
        </Link>
      </p>
    );
  }
  if (!d) return null;

  const apiBase = cliSuggestedApiBase();
  const installCmd = "cargo install --path crates/deploywerk-cli";
  const loginCmd = `deploywerk --base-url ${JSON.stringify(apiBase)} auth login --email ${JSON.stringify(d.user.email)}`;

  async function copyText(label: string, text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toastSuccess(`Copied ${label}`);
    } catch {
      toastError("Copy failed");
    }
  }

  return (
    <div className="space-y-8">
      <div>
        <Link
          to="/admin/users"
          className="inline-flex items-center gap-1 text-sm text-violet-700 hover:underline"
        >
          <ArrowLeft className="h-4 w-4" strokeWidth={1.75} aria-hidden />
          Users
        </Link>
        <h1 className="mt-2 text-2xl font-semibold text-slate-900">{d.user.email}</h1>
        <p className="mt-1 text-sm text-slate-600">{d.user.name ?? "No display name"}</p>
      </div>
      {err && <p className="text-sm text-red-600">{err}</p>}

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="flex items-center gap-2 text-lg font-semibold text-slate-900">
          <ShieldCheck className="h-5 w-5 text-violet-600" strokeWidth={1.75} aria-hidden />
          Super admin
        </h2>
        <p className="mt-1 text-sm text-slate-600">Grant or revoke operator access to this console.</p>
        <label className="mt-4 flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={d.user.is_platform_admin}
            disabled={saving}
            onChange={(e) => void togglePlatformAdmin(e.target.checked)}
          />
          <span>Super administrator</span>
        </label>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="flex items-center gap-2 text-lg font-semibold text-slate-900">
          <Terminal className="h-5 w-5 text-violet-600" strokeWidth={1.75} aria-hidden />
          CLI for this user
        </h2>
        <p className="mt-1 text-sm text-slate-600">
          Install the <code className="rounded bg-slate-100 px-1">deploywerk</code> binary from the repo. The login          command uses this user&apos;s email; only works if they can use password auth (not SSO-only). Otherwise create
          an{" "}
          <Link to="/app/settings/tokens" className="font-medium text-violet-700 hover:underline">
            API token
          </Link>{" "}
          while signed in as them, or use <code className="rounded bg-slate-100 px-1">DEPLOYWERK_API_URL</code> with a
          token in config.
        </p>
        <p className="mt-2 text-xs text-slate-500">
          Suggested API base for this browser session: <code className="rounded bg-slate-50 px-1">{apiBase}</code>{" "}
          (from <code className="rounded bg-slate-50 px-1">VITE_API_URL</code> when set, else current web origin).
        </p>
        <div className="mt-4 space-y-3">
          <div>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-xs font-semibold uppercase tracking-wide text-slate-500">Install (from repo)</span>
              <button
                type="button"
                className="text-xs font-medium text-violet-700 hover:underline"
                onClick={() => void copyText("install command", installCmd)}
              >
                Copy
              </button>
            </div>
            <pre className="mt-1 overflow-x-auto rounded-lg bg-slate-950 p-3 text-xs text-slate-100">{installCmd}</pre>
          </div>
          <div>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-xs font-semibold uppercase tracking-wide text-slate-500">Login as this email</span>
              <button
                type="button"
                className="text-xs font-medium text-violet-700 hover:underline"
                onClick={() => void copyText("login command", loginCmd)}
              >
                Copy
              </button>
            </div>
            <pre className="mt-1 overflow-x-auto rounded-lg bg-slate-950 p-3 text-xs text-slate-100">{loginCmd}</pre>
          </div>
        </div>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Organizations</h2>
        <ul className="mt-3 space-y-2 text-sm">
          {d.organization_memberships.map((o) => (
            <li key={o.organization_id}>
              <AdminViewLink to={`/admin/organizations/${o.organization_id}`} label={o.org_name} icon={Eye} />{" "}
              <span className="text-slate-500">
                ({o.role}) · {o.org_slug}
              </span>
            </li>
          ))}
          {d.organization_memberships.length === 0 && <li className="text-slate-500">None</li>}
        </ul>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Teams</h2>
        <ul className="mt-3 space-y-2 text-sm">
          {d.team_memberships.map((t) => (
            <li key={t.team_id}>
              <AdminViewLink to={`/admin/teams/${t.team_id}`} label={t.team_name} icon={Eye} />{" "}
              <span className="text-slate-500">
                ({t.role}) · {t.team_slug}
              </span>
            </li>
          ))}
          {d.team_memberships.length === 0 && <li className="text-slate-500">None</li>}
        </ul>
      </section>
    </div>
  );
}
