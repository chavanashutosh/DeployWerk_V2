import { FormEvent, useEffect, useState } from "react";
import { KeyRound, Trash2 } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { apiFetch, type Team, type TeamSecretMeta } from "@/api";

export function TeamSecretsSettingsPage() {
  const { teamId = "" } = useParams();
  const [teams, setTeams] = useState<Team[]>([]);
  const [rows, setRows] = useState<TeamSecretMeta[] | null>(null);
  const [versionsByName, setVersionsByName] = useState<
    Record<string, { version: number; created_at: string; created_by_user_id?: string | null }[]>
  >({});
  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const team = teams.find((t) => t.id === teamId);
  const canMutate = team?.role === "admin" || team?.role === "owner";

  useEffect(() => {
    let c = false;
    (async () => {
      try {
        const t = await apiFetch<Team[]>("/api/v1/teams");
        if (!c) setTeams(t);
      } catch {
        if (!c) setTeams([]);
      }
    })();
    return () => {
      c = true;
    };
  }, []);

  async function load() {
    if (!teamId) return;
    const list = await apiFetch<TeamSecretMeta[]>(`/api/v1/teams/${teamId}/secrets`);
    setRows(list);
    setVersionsByName({});
  }

  useEffect(() => {
    if (!teamId) return;
    let c = false;
    (async () => {
      try {
        await load();
        if (!c) setErr(null);
      } catch (e) {
        if (!c) {
          setErr(e instanceof Error ? e.message : "Failed to load secrets");
          setRows(null);
        }
      }
    })();
    return () => {
      c = true;
    };
  }, [teamId]);

  async function onUpsert(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !canMutate) return;
    const n = name.trim();
    if (!n || n.length > 128) {
      setErr("Name must be 1–128 characters.");
      return;
    }
    setPending(true);
    setErr(null);
    setMsg(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/secrets`, {
        method: "POST",
        body: JSON.stringify({ name: n, value }),
      });
      setName("");
      setValue("");
      setMsg("Secret saved.");
      await load();
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : "Save failed");
    } finally {
      setPending(false);
    }
  }

  async function onDelete(secretName: string) {
    if (!teamId || !canMutate) return;
    if (!confirm(`Delete secret “${secretName}”? Apps referencing dw_secret:${secretName} will fail until fixed.`)) {
      return;
    }
    setErr(null);
    setMsg(null);
    try {
      const path = `/api/v1/teams/${teamId}/secrets/${encodeURIComponent(secretName)}`;
      await apiFetch(path, { method: "DELETE" });
      setMsg("Secret removed.");
      await load();
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : "Delete failed");
    }
  }

  async function loadVersions(secretName: string) {
    if (!teamId) return;
    if (versionsByName[secretName]) return;
    try {
      const rows2 = await apiFetch<
        { version: number; created_at: string; created_by_user_id?: string | null }[]
      >(`/api/v1/teams/${teamId}/secrets/${encodeURIComponent(secretName)}/versions`);
      setVersionsByName((m) => ({ ...m, [secretName]: rows2 }));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Failed to load versions");
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-start gap-3">
        <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-brand-100 text-brand-800">
          <KeyRound className="h-5 w-5" strokeWidth={1.75} />
        </span>
        <div>
          <h2 className="text-lg font-semibold text-slate-900">Team secrets</h2>
          <p className="mt-1 text-sm text-slate-600">
            Encrypted values scoped to this team. Reference them from application environment variables as{" "}
            <code className="rounded bg-slate-100 px-1 font-mono text-xs">dw_secret:NAME</code> (set under{" "}
            <Link
              to={`/app/teams/${teamId}/projects`}
              className="font-medium text-brand-700 underline-offset-2 hover:underline"
            >
              Projects
            </Link>
            → application env).
          </p>
          <p className="mt-1 text-xs text-slate-500">
            You can pin a specific secret version as{" "}
            <code className="rounded bg-slate-100 px-1 font-mono text-xs">dw_secret:NAME@VERSION</code>.
          </p>
        </div>
      </div>
      {msg && <p className="text-sm text-emerald-700">{msg}</p>}
      {err && <p className="text-sm text-red-600">{err}</p>}

      {!canMutate && (
        <p className="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
          Only team owners and admins can create or delete secrets. You can still view names and timestamps.
        </p>
      )}

      <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h3 className="text-sm font-semibold text-slate-900">Stored secrets</h3>
        {rows === null ? (
          <p className="mt-2 text-sm text-slate-500">Loading…</p>
        ) : rows.length === 0 ? (
          <p className="mt-2 text-sm text-slate-600">No secrets yet.</p>
        ) : (
          <ul className="mt-3 divide-y divide-slate-100">
            {rows.map((r) => (
              <li key={r.name} className="py-3 first:pt-0">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                  <p className="font-mono text-sm font-medium text-slate-900">{r.name}</p>
                  <p className="text-xs text-slate-500">
                    Updated {new Date(r.updated_at).toLocaleString()}
                  </p>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <button
                      type="button"
                      onClick={() => void loadVersions(r.name)}
                      className="inline-flex items-center gap-1 rounded-lg border border-slate-200 px-2 py-1 text-xs font-medium text-slate-800 hover:bg-slate-50"
                    >
                      Versions
                    </button>
                    {canMutate && (
                      <button
                        type="button"
                        onClick={() => void onDelete(r.name)}
                        className="inline-flex items-center gap-1 rounded-lg border border-red-200 px-2 py-1 text-xs font-medium text-red-800 hover:bg-red-50"
                      >
                        <Trash2 className="h-3.5 w-3.5" strokeWidth={1.75} />
                        Delete
                      </button>
                    )}
                  </div>
                </div>
                {versionsByName[r.name] && (
                  <div className="mt-2 rounded-lg border border-slate-100 bg-slate-50 px-3 py-2">
                    <p className="text-xs font-medium uppercase tracking-wider text-slate-500">
                      Versions (newest first)
                    </p>
                    <ul className="mt-2 space-y-1 text-xs text-slate-700">
                      {versionsByName[r.name].slice(0, 10).map((v) => (
                        <li key={v.version} className="flex flex-wrap items-center justify-between gap-2">
                          <span className="font-mono">v{v.version}</span>
                          <span className="text-slate-500">
                            {new Date(v.created_at).toLocaleString()}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      {canMutate && (
        <form onSubmit={onUpsert} className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <h3 className="text-sm font-semibold text-slate-900">Add or rotate</h3>
          <p className="mt-1 text-xs text-slate-500">
            Values are encrypted server-side and never shown again after save.
          </p>
          <div className="mt-4 grid max-w-xl gap-3">
            <label className="text-sm">
              <span className="text-slate-600">Name</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="mt-1 block w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                placeholder="DATABASE_URL"
                required
                maxLength={128}
                autoComplete="off"
              />
            </label>
            <label className="text-sm">
              <span className="text-slate-600">Value</span>
              <input
                type="password"
                value={value}
                onChange={(e) => setValue(e.target.value)}
                className="mt-1 block w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                placeholder="secret value"
                required
                autoComplete="new-password"
              />
            </label>
          </div>
          <button
            type="submit"
            disabled={pending}
            className="mt-4 rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700 disabled:opacity-60"
          >
            {pending ? "Saving…" : "Save secret"}
          </button>
        </form>
      )}
    </div>
  );
}
