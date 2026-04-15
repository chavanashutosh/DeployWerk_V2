import { useEffect, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { Layers } from "lucide-react";
import { apiFetch, type Environment, type Project, type Team } from "@/api";
import { EmptyState, InlineError, LoadingBlock, PageHeader } from "@/components/ui";

async function loadEnvs(teamId: string, projectId: string): Promise<Environment[]> {
  return apiFetch<Environment[]>(
    `/api/v1/teams/${teamId}/projects/${projectId}/environments`,
  );
}

export function EnvironmentsPage() {
  const { teamId = "", projectId = "" } = useParams();
  const [envs, setEnvs] = useState<Environment[] | null>(null);
  const [teams, setTeams] = useState<Team[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [pending, setPending] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editSlug, setEditSlug] = useState("");
  const [editDeployLocked, setEditDeployLocked] = useState(false);
  const [editLockReason, setEditLockReason] = useState("");
  const [editScheduleJson, setEditScheduleJson] = useState("");
  const [editPending, setEditPending] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [project, setProject] = useState<Project | null>(null);

  const team = teams.find((t) => t.id === teamId);
  const canMutate = team?.role === "admin" || team?.role === "owner";

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const t = await apiFetch<Team[]>("/api/v1/teams");
        if (!cancelled) setTeams(t);
      } catch {
        if (!cancelled) setTeams([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!teamId || !projectId) return;
    let cancelled = false;
    (async () => {
      try {
        const p = await apiFetch<Project>(`/api/v1/teams/${teamId}/projects/${projectId}`);
        if (!cancelled) setProject(p);
      } catch {
        if (!cancelled) setProject(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId, projectId]);

  useEffect(() => {
    if (!teamId || !projectId) return;
    let cancelled = false;
    (async () => {
      try {
        const list = await loadEnvs(teamId, projectId);
        if (!cancelled) {
          setEnvs(list);
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load");
          setEnvs(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId, projectId]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !projectId || !name.trim()) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(
        `/api/v1/teams/${teamId}/projects/${projectId}/environments`,
        {
          method: "POST",
          body: JSON.stringify({ name: name.trim() }),
        },
      );
      setName("");
      setEnvs(await loadEnvs(teamId, projectId));
    } catch (err2) {
      setErr(err2 instanceof Error ? err2.message : "Create failed");
    } finally {
      setPending(false);
    }
  }

  function startEdit(env: Environment) {
    setEditingId(env.id);
    setEditName(env.name);
    setEditSlug(env.slug);
    setEditDeployLocked(!!env.deploy_locked);
    setEditLockReason(env.deploy_lock_reason ?? "");
    setEditScheduleJson(env.deploy_schedule_json ?? "");
  }

  function cancelEdit() {
    setEditingId(null);
  }

  async function onSaveEdit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !projectId || !editingId || !editName.trim()) return;
    setEditPending(true);
    setErr(null);
    try {
      await apiFetch(
        `/api/v1/teams/${teamId}/projects/${projectId}/environments/${editingId}`,
        {
          method: "PATCH",
          body: JSON.stringify({
            name: editName.trim(),
            slug: editSlug.trim() || undefined,
            deploy_locked: editDeployLocked,
            deploy_lock_reason: editLockReason.trim() || null,
            deploy_schedule_json: editScheduleJson.trim() || null,
          }),
        },
      );
      setEditingId(null);
      setEnvs(await loadEnvs(teamId, projectId));
    } catch (err2) {
      setErr(err2 instanceof Error ? err2.message : "Update failed");
    } finally {
      setEditPending(false);
    }
  }

  async function onConfirmDelete() {
    if (!teamId || !projectId || !deleteId) return;
    setErr(null);
    try {
      await apiFetch(
        `/api/v1/teams/${teamId}/projects/${projectId}/environments/${deleteId}`,
        { method: "DELETE" },
      );
      setDeleteId(null);
      if (editingId === deleteId) setEditingId(null);
      setEnvs(await loadEnvs(teamId, projectId));
    } catch (err2) {
      setErr(err2 instanceof Error ? err2.message : "Delete failed");
    }
  }

  if (!teamId || !projectId) {
    return <p className="dw-muted">Missing team or project.</p>;
  }

  return (
    <div className="space-y-8">
      <PageHeader
        icon={<Layers className="h-6 w-6" strokeWidth={1.75} />}
        title="Environments"
        description={
          <>
            <Link to={`/app/teams/${teamId}/projects`} className="font-medium text-brand-600 hover:text-brand-700">
              Projects
            </Link>
            <span className="mx-1.5 text-slate-400">/</span>
            <span className="font-medium text-slate-800">{project?.name ?? "…"}</span>
            <span className="dw-muted"> · isolate production, staging, and previews.</span>
          </>
        }
      />

      <InlineError message={err} />

      {envs === null && !err && <LoadingBlock label="Loading environments…" />}

      {envs && envs.length === 0 && (
        <EmptyState icon={Layers} title="No environments yet">
          {canMutate
            ? "Add production and staging environments below, then attach applications to each."
            : "No environments in this project yet. Ask a team owner or admin to create them."}
        </EmptyState>
      )}

      {envs && envs.length > 0 && (
        <div className="dw-card p-6 sm:p-8">
          <h2 className="text-lg font-semibold text-slate-900">Your environments</h2>
          <ul className="mt-4 divide-y divide-slate-100">
            {envs.map((env) => (
              <li key={env.id} className="py-4 first:pt-0">
                {editingId === env.id && canMutate ? (
                  <form className="space-y-3" onSubmit={(e) => void onSaveEdit(e)}>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`en-${env.id}`}>
                        Name
                      </label>
                      <input
                        id={`en-${env.id}`}
                        value={editName}
                        onChange={(e) => setEditName(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                        required
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`es-${env.id}`}>
                        Slug
                      </label>
                      <input
                        id={`es-${env.id}`}
                        value={editSlug}
                        onChange={(e) => setEditSlug(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                      />
                    </div>
                    <label className="flex items-center gap-2 text-sm text-slate-700">
                      <input
                        type="checkbox"
                        checked={editDeployLocked}
                        onChange={(e) => setEditDeployLocked(e.target.checked)}
                      />
                      Deploy lock (block new deploys)
                    </label>
                    <div>
                      <label className="text-xs font-medium text-slate-500">Lock reason (optional)</label>
                      <input
                        value={editLockReason}
                        onChange={(e) => setEditLockReason(e.target.value)}
                        className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 text-sm"
                        placeholder="e.g. incident response"
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-slate-500">
                        Deploy schedule JSON (optional, UTC hours)
                      </label>
                      <textarea
                        value={editScheduleJson}
                        onChange={(e) => setEditScheduleJson(e.target.value)}
                        rows={3}
                        placeholder='{"utc_start_hour":9,"utc_end_hour":18,"weekdays_only":true}'
                        className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-xs"
                      />
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <button
                        type="submit"
                        disabled={editPending}
                        className="rounded-lg bg-brand-600 px-3 py-1.5 text-sm font-semibold text-white hover:bg-brand-700 disabled:opacity-60"
                      >
                        {editPending ? "Saving…" : "Save"}
                      </button>
                      <button
                        type="button"
                        onClick={cancelEdit}
                        className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm text-slate-700"
                      >
                        Cancel
                      </button>
                    </div>
                  </form>
                ) : (
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <div>
                      <p className="font-medium text-slate-900">{env.name}</p>
                      <p className="text-sm font-mono text-slate-500">{env.slug}</p>
                      {env.deploy_locked && (
                        <p className="mt-1 text-xs font-medium text-amber-800">
                          Locked
                          {env.deploy_lock_reason ? `: ${env.deploy_lock_reason}` : ""}
                        </p>
                      )}
                      <Link
                        to={`/app/teams/${teamId}/projects/${projectId}/environments/${env.id}/applications`}
                        className="mt-2 inline-block text-sm font-medium text-brand-600 hover:text-brand-700"
                      >
                        Applications →
                      </Link>
                    </div>
                    {canMutate && (
                      <div className="flex gap-2">
                        <button
                          type="button"
                          onClick={() => startEdit(env)}
                          className="text-sm font-medium text-slate-600 hover:text-slate-900"
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          onClick={() => setDeleteId(env.id)}
                          className="text-sm font-medium text-red-600 hover:text-red-700"
                        >
                          Delete
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      {deleteId && (
        <div
          className="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4"
          role="dialog"
          aria-modal="true"
        >
          <div className="max-w-md rounded-xl bg-white p-6 shadow-lg">
            <p className="text-sm text-slate-800">Delete this environment? This cannot be undone.</p>
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setDeleteId(null)}
                className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => void onConfirmDelete()}
                className="rounded-lg bg-red-600 px-3 py-2 text-sm font-semibold text-white hover:bg-red-700"
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}

      {canMutate && (
        <div className="dw-card p-6 sm:p-8">
          <h2 className="text-lg font-semibold text-slate-900">New environment</h2>
          <p className="mt-1 text-sm text-slate-600">
            Optional: set deploy locks and schedules when editing an environment.
          </p>
          <form className="mt-4 flex flex-wrap items-end gap-3" onSubmit={onCreate}>
            <div className="min-w-[200px] flex-1">
              <label className="block text-sm font-medium text-slate-700" htmlFor="ename">
                Name
              </label>
              <input
                id="ename"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="dw-input mt-1"
                required
              />
            </div>
            <button type="submit" disabled={pending} className="dw-btn-primary">
              {pending ? "Creating…" : "Create"}
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
