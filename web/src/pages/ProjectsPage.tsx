import { useEffect, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { FolderKanban, Pencil, Plus, Save, Trash2, X } from "lucide-react";
import { apiFetch, type Project, type Team } from "@/api";
import { EmptyState, InlineError, LoadingBlock, PageHeader } from "@/components/ui";

async function loadProjects(teamId: string): Promise<Project[]> {
  return apiFetch<Project[]>(`/api/v1/teams/${teamId}/projects`);
}

export function ProjectsPage() {
  const { teamId = "" } = useParams();
  const [projects, setProjects] = useState<Project[] | null>(null);
  const [teams, setTeams] = useState<Team[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [pending, setPending] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editSlug, setEditSlug] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [editPending, setEditPending] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);

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
    if (!teamId) return;
    let cancelled = false;
    (async () => {
      try {
        const list = await loadProjects(teamId);
        if (!cancelled) {
          setProjects(list);
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load");
          setProjects(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !name.trim()) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/projects`, {
        method: "POST",
        body: JSON.stringify({ name: name.trim() }),
      });
      setName("");
      setProjects(await loadProjects(teamId));
    } catch (err2) {
      setErr(err2 instanceof Error ? err2.message : "Create failed");
    } finally {
      setPending(false);
    }
  }

  function startEdit(p: Project) {
    setEditingId(p.id);
    setEditName(p.name);
    setEditSlug(p.slug);
    setEditDescription(p.description ?? "");
  }

  function cancelEdit() {
    setEditingId(null);
  }

  async function onSaveEdit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !editingId || !editName.trim()) return;
    setEditPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/projects/${editingId}`, {
        method: "PATCH",
        body: JSON.stringify({
          name: editName.trim(),
          slug: editSlug.trim() || undefined,
          description: editDescription.trim() || null,
        }),
      });
      setEditingId(null);
      setProjects(await loadProjects(teamId));
    } catch (err2) {
      setErr(err2 instanceof Error ? err2.message : "Update failed");
    } finally {
      setEditPending(false);
    }
  }

  async function onConfirmDelete() {
    if (!teamId || !deleteId) return;
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/projects/${deleteId}`, {
        method: "DELETE",
      });
      setDeleteId(null);
      if (editingId === deleteId) setEditingId(null);
      setProjects(await loadProjects(teamId));
    } catch (err2) {
      setErr(err2 instanceof Error ? err2.message : "Delete failed");
    }
  }

  if (!teamId) {
    return <p className="dw-muted">Select a team from the sidebar.</p>;
  }

  return (
    <div className="space-y-8">
      <PageHeader
        icon={<FolderKanban className="h-6 w-6" strokeWidth={1.75} />}
        title="Projects"
        description={
          <>
            Group applications under <span className="font-medium text-slate-800">{team?.name ?? "this team"}</span>.
            Each project holds one or more environments (e.g. production, staging).
          </>
        }
      />

      <InlineError message={err} />

      {projects === null && !err && <LoadingBlock label="Loading projects…" />}

      {projects && projects.length === 0 && (
        <EmptyState icon={FolderKanban} title="No projects yet">
          {canMutate
            ? "Create your first project below, then add environments and applications."
            : "Ask a team owner or admin to create a project. You can browse resources once they exist."}
        </EmptyState>
      )}

      {projects && projects.length > 0 && (
        <div className="dw-card p-6 sm:p-8">
          <h2 className="text-lg font-semibold text-slate-900">Your projects</h2>
          <ul className="mt-4 divide-y divide-slate-100">
            {projects.map((p) => (
              <li key={p.id} className="py-4 first:pt-0">
                {editingId === p.id && canMutate ? (
                  <form className="space-y-3" onSubmit={(e) => void onSaveEdit(e)}>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`en-${p.id}`}>
                        Name
                      </label>
                      <input
                        id={`en-${p.id}`}
                        value={editName}
                        onChange={(e) => setEditName(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                        required
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`es-${p.id}`}>
                        Slug
                      </label>
                      <input
                        id={`es-${p.id}`}
                        value={editSlug}
                        onChange={(e) => setEditSlug(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`ed-${p.id}`}>
                        Description
                      </label>
                      <input
                        id={`ed-${p.id}`}
                        value={editDescription}
                        onChange={(e) => setEditDescription(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                      />
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <button
                        type="submit"
                        disabled={editPending}
                        className="inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-3 py-1.5 text-sm font-semibold text-white hover:bg-brand-700 disabled:opacity-60"
                      >
                        <Save className="h-4 w-4" strokeWidth={1.75} />
                        {editPending ? "Saving…" : "Save"}
                      </button>
                      <button
                        type="button"
                        onClick={cancelEdit}
                        className="inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-3 py-1.5 text-sm text-slate-700"
                      >
                        <X className="h-4 w-4" strokeWidth={1.75} />
                        Cancel
                      </button>
                    </div>
                  </form>
                ) : (
                  <>
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <div>
                        <Link
                          to={`/app/teams/${teamId}/projects/${p.id}/environments`}
                          className="font-medium text-brand-700 hover:text-brand-800"
                        >
                          {p.name}
                        </Link>
                        <p className="text-sm text-slate-500">
                          <span className="font-mono">{p.slug}</span>
                        </p>
                        {p.description && (
                          <p className="mt-1 text-sm text-slate-600">{p.description}</p>
                        )}
                      </div>
                      {canMutate && (
                        <div className="flex gap-2">
                          <button
                            type="button"
                            onClick={() => startEdit(p)}
                            className="inline-flex items-center gap-1 text-sm font-medium text-slate-600 hover:text-slate-900"
                          >
                            <Pencil className="h-3.5 w-3.5" strokeWidth={1.75} />
                            Edit
                          </button>
                          <button
                            type="button"
                            onClick={() => setDeleteId(p.id)}
                            className="inline-flex items-center gap-1 text-sm font-medium text-red-600 hover:text-red-700"
                          >
                            <Trash2 className="h-3.5 w-3.5" strokeWidth={1.75} />
                            Delete
                          </button>
                        </div>
                      )}
                    </div>
                  </>
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
            <p className="text-sm text-slate-800">
              Delete this project and all its environments? This cannot be undone.
            </p>
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setDeleteId(null)}
                className="inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-3 py-2 text-sm"
              >
                <X className="h-4 w-4" strokeWidth={1.75} />
                Cancel
              </button>
              <button
                type="button"
                onClick={() => void onConfirmDelete()}
                className="inline-flex items-center gap-1.5 rounded-lg bg-red-600 px-3 py-2 text-sm font-semibold text-white hover:bg-red-700"
              >
                <Trash2 className="h-4 w-4" strokeWidth={1.75} />
                Delete
              </button>
            </div>
          </div>
        </div>
      )}

      {canMutate && (
        <div className="dw-card p-6 sm:p-8">
          <h2 className="text-lg font-semibold text-slate-900">New project</h2>
          <p className="mt-1 text-sm text-slate-600">Slug is generated from the name unless you edit it later.</p>
          <form className="mt-4 flex flex-wrap items-end gap-3" onSubmit={onCreate}>
            <div className="min-w-[200px] flex-1">
              <label className="block text-sm font-medium text-slate-700" htmlFor="pname">
                Name
              </label>
              <input
                id="pname"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="dw-input mt-1"
                required
              />
            </div>
            <button type="submit" disabled={pending} className="dw-btn-primary gap-2">
              <Plus className="h-4 w-4" strokeWidth={1.75} />
              {pending ? "Creating…" : "Create"}
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
