import { ArrowLeft, Plus } from "lucide-react";
import { FormEvent, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { apiFetch } from "@/api";
import { toastError, toastSuccess } from "@/toast";
import { AdminTableWrap, AdminThead, formatAdminListError } from "./adminUi";

type EntRow = {
  feature_key: string;
  enabled: boolean;
  source: string;
  expires_at: string | null;
  updated_at: string;
  default_on: boolean;
  effective: boolean;
};

type DefRow = {
  feature_key: string;
  description: string;
  default_on: boolean;
};

export function AdminTeamEntitlementsPage() {
  const { teamId = "" } = useParams();
  const [defs, setDefs] = useState<DefRow[]>([]);
  const [rows, setRows] = useState<EntRow[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [feature_key, setKey] = useState("");
  const [enabled, setEnabled] = useState(true);
  const [source, setSource] = useState("manual");
  const [expires_at, setExpires] = useState("");

  async function load() {
    if (!teamId) return;
    try {
      const [d, e] = await Promise.all([
        apiFetch<DefRow[]>("/api/v1/admin/features"),
        apiFetch<EntRow[]>(`/api/v1/admin/teams/${teamId}/entitlements`),
      ]);
      setDefs(d);
      setRows(e);
      if (d.length && !feature_key) setKey(d[0].feature_key);
      setErr(null);
    } catch (ex) {
      setErr(formatAdminListError(ex));
    }
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function save(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !feature_key) return;
    try {
      await apiFetch(`/api/v1/admin/teams/${teamId}/entitlements`, {
        method: "POST",
        body: JSON.stringify({
          feature_key,
          enabled,
          source,
          expires_at: expires_at ? new Date(expires_at).toISOString() : null,
        }),
      });
      setExpires("");
      toastSuccess("Entitlement saved");
      await load();
    } catch (ex) {
      const m = formatAdminListError(ex);
      setErr(m);
      toastError(m);
    }
  }

  return (
    <div className="space-y-8">
      <div>
        <Link
          to={`/admin/teams/${teamId}`}
          className="inline-flex items-center gap-1 text-sm text-violet-700 hover:underline"
        >
          <ArrowLeft className="h-4 w-4" strokeWidth={1.75} aria-hidden />
          Team
        </Link>
        <h1 className="mt-2 text-2xl font-semibold text-slate-900">Entitlements</h1>
        <p className="mt-1 text-sm text-slate-600">
          Platform product gates (AI Gateway, RUM, …). Overrides defaults for this team.
        </p>
      </div>

      {err && <p className="text-sm text-red-600">{err}</p>}

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Definitions</h2>
        <ul className="mt-3 space-y-2 text-sm text-slate-600">
          {defs.map((d) => (
            <li key={d.feature_key}>
              <code className="rounded bg-slate-100 px-1">{d.feature_key}</code> — {d.description}{" "}
              <span className="text-slate-400">(default {d.default_on ? "on" : "off"})</span>
            </li>
          ))}
        </ul>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Effective per feature</h2>
        <AdminTableWrap className="mt-4">
          <table className="w-full min-w-[520px] text-sm">
            <AdminThead>
              <tr>
                <th className="px-4 py-3 font-medium">Feature</th>
                <th className="px-4 py-3 font-medium">Source</th>
                <th className="px-4 py-3 font-medium">Row enabled</th>
                <th className="px-4 py-3 font-medium">Effective</th>
                <th className="px-4 py-3 font-medium">Expires</th>
              </tr>
            </AdminThead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.feature_key} className="border-b border-slate-100 even:bg-slate-50/40">
                  <td className="px-4 py-2 font-mono text-xs">{r.feature_key}</td>
                  <td className="px-4 py-2">{r.source}</td>
                  <td className="px-4 py-2">{r.enabled ? "yes" : "no"}</td>
                  <td className="px-4 py-2">{r.effective ? "yes" : "no"}</td>
                  <td className="px-4 py-2 text-slate-600">
                    {r.expires_at ? new Date(r.expires_at).toLocaleString() : "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {rows.length === 0 && <p className="px-4 py-6 text-sm text-slate-500">No entitlement rows yet.</p>}
        </AdminTableWrap>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Upsert override</h2>
        <form onSubmit={save} className="mt-4 max-w-lg space-y-3">
          <label className="block text-sm">
            <span className="text-slate-600">Feature</span>
            <select
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
              value={feature_key}
              onChange={(e) => setKey(e.target.value)}
            >
              {defs.map((d) => (
                <option key={d.feature_key} value={d.feature_key}>
                  {d.feature_key}
                </option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
            Enabled
          </label>
          <label className="block text-sm">
            <span className="text-slate-600">Source</span>
            <select
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
              value={source}
              onChange={(e) => setSource(e.target.value)}
            >
              <option value="manual">manual</option>
              <option value="plan">plan</option>
              <option value="trial">trial</option>
            </select>
          </label>
          <label className="block text-sm">
            <span className="text-slate-600">Expires at (optional, local → UTC)</span>
            <input
              type="datetime-local"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
              value={expires_at}
              onChange={(e) => setExpires(e.target.value)}
            />
          </label>
          <button
            type="submit"
            className="inline-flex items-center gap-2 rounded-lg bg-violet-600 px-4 py-2 text-sm font-medium text-white hover:bg-violet-700"
          >
            <Plus className="h-4 w-4" strokeWidth={1.75} aria-hidden />
            Save entitlement
          </button>
        </form>
      </section>
    </div>
  );
}
