import { useEffect, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { apiFetch, type Bootstrap, type Destination, type Server, type Team } from "@/api";

export function DestinationsPage() {
  const { teamId = "" } = useParams();
  const [destinations, setDestinations] = useState<Destination[] | null>(null);
  const [servers, setServers] = useState<Server[]>([]);
  const [teams, setTeams] = useState<Team[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [serverId, setServerId] = useState("");
  const [description, setDescription] = useState("");
  const [pending, setPending] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editServerId, setEditServerId] = useState("");
  const [editDesc, setEditDesc] = useState("");
  const [editPending, setEditPending] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [editingKind, setEditingKind] = useState<Destination["kind"] | null>(null);

  const team = teams.find((t) => t.id === teamId);
  const canMutate = team?.role === "admin" || team?.role === "owner";

  async function loadAll() {
    if (!teamId) return;
    const [d, s] = await Promise.all([
      apiFetch<Destination[]>(`/api/v1/teams/${teamId}/destinations`),
      apiFetch<Server[]>(`/api/v1/teams/${teamId}/servers`),
    ]);
    setDestinations(d);
    setServers(s);
  }

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [t, b] = await Promise.all([
          apiFetch<Team[]>("/api/v1/teams"),
          apiFetch<Bootstrap>("/api/v1/bootstrap"),
        ]);
        if (!cancelled) {
          setTeams(t);
          setBootstrap(b);
        }
      } catch {
        if (!cancelled) {
          setTeams([]);
          setBootstrap(null);
        }
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
        await loadAll();
        if (!cancelled) setErr(null);
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load");
          setDestinations(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !name.trim() || !slug.trim() || !serverId) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/destinations`, {
        method: "POST",
        body: JSON.stringify({
          server_id: serverId,
          name: name.trim(),
          slug: slug.trim().toLowerCase(),
          kind: "docker_standalone",
          description: description.trim() || null,
        }),
      });
      setName("");
      setSlug("");
      setDescription("");
      await loadAll();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Create failed");
    } finally {
      setPending(false);
    }
  }

  function startEdit(d: Destination) {
    setEditingId(d.id);
    setEditingKind(d.kind);
    setEditName(d.name);
    setEditServerId(d.server_id ?? "");
    setEditDesc(d.description ?? "");
  }

  function cancelEdit() {
    setEditingId(null);
    setEditingKind(null);
  }

  async function onSaveEdit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !editingId || !editName.trim()) return;
    if (editingKind === "docker_standalone" && !editServerId) return;
    setEditPending(true);
    setErr(null);
    try {
      const body =
        editingKind === "docker_platform"
          ? { name: editName.trim(), description: editDesc.trim() || null }
          : {
              name: editName.trim(),
              server_id: editServerId,
              description: editDesc.trim() || null,
            };
      await apiFetch(`/api/v1/teams/${teamId}/destinations/${editingId}`, {
        method: "PATCH",
        body: JSON.stringify(body),
      });
      setEditingId(null);
      setEditingKind(null);
      await loadAll();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Save failed");
    } finally {
      setEditPending(false);
    }
  }

  async function onConfirmDelete() {
    if (!teamId || !deleteId) return;
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/destinations/${deleteId}`, {
        method: "DELETE",
      });
      if (editingId === deleteId) setEditingId(null);
      setDeleteId(null);
      await loadAll();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Delete failed");
    }
  }

  if (!teamId) {
    return <p className="text-slate-600">Missing team.</p>;
  }

  return (
    <div className="space-y-8">
      <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
        <h1 className="text-2xl font-bold text-slate-900">Destinations</h1>
        <p className="mt-2 text-sm text-slate-600">
          <Link
            to={`/app/teams/${teamId}/servers`}
            className="font-medium text-brand-600 hover:text-brand-700"
          >
            ← Servers
          </Link>
          <span className="mx-2 text-slate-400">·</span>
          Docker on your SSH servers, or on the DeployWerk API host when platform Docker is enabled.
        </p>
        {bootstrap?.platform_docker_enabled && (
          <p className="mt-3 rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-sm text-sky-900">
            <strong>Platform destination.</strong> Each team gets a built-in &quot;Platform (API host)&quot; target when
            you open this page. Deploys run <code className="rounded bg-white px-1">docker</code> on the same machine as
            the API (operator must expose the Docker socket to the API process).
          </p>
        )}
        {err && <p className="mt-4 text-sm text-red-600">{err}</p>}
        {destinations && destinations.length === 0 && (
          <p className="mt-4 text-sm text-slate-600">No destinations yet.</p>
        )}
        {destinations && destinations.length > 0 && (
          <ul className="mt-6 divide-y divide-slate-100">
            {destinations.map((d) => (
              <li key={d.id} className="py-4 first:pt-0">
                {editingId === d.id && canMutate ? (
                  <form className="space-y-3" onSubmit={(e) => void onSaveEdit(e)}>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`dn-${d.id}`}>
                        Name
                      </label>
                      <input
                        id={`dn-${d.id}`}
                        value={editName}
                        onChange={(e) => setEditName(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                        required
                      />
                    </div>
                    {editingKind !== "docker_platform" && (
                      <div>
                        <label className="text-xs font-medium text-slate-500" htmlFor={`ds-${d.id}`}>
                          Server
                        </label>
                        <select
                          id={`ds-${d.id}`}
                          value={editServerId}
                          onChange={(e) => setEditServerId(e.target.value)}
                          className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                          required
                        >
                          {servers.map((s) => (
                            <option key={s.id} value={s.id}>
                              {s.name} ({s.host})
                            </option>
                          ))}
                        </select>
                      </div>
                    )}
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`dd-${d.id}`}>
                        Description
                      </label>
                      <input
                        id={`dd-${d.id}`}
                        value={editDesc}
                        onChange={(e) => setEditDesc(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
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
                      <p className="font-medium text-slate-900">{d.name}</p>
                      <p className="text-sm font-mono text-slate-500">{d.slug}</p>
                      <p className="mt-1 text-xs text-slate-500">
                        {d.kind === "docker_platform" ? (
                          <>Target: DeployWerk API host (local Docker)</>
                        ) : (
                          <>
                            Server:{" "}
                            {d.server_id
                              ? (servers.find((s) => s.id === d.server_id)?.name ?? d.server_id)
                              : "—"}
                          </>
                        )}
                        <span className="ml-2 rounded bg-slate-100 px-1.5 py-0.5 font-mono text-[10px] text-slate-600">
                          {d.kind}
                        </span>
                      </p>
                      {d.description && (
                        <p className="mt-1 text-sm text-slate-600">{d.description}</p>
                      )}
                    </div>
                    {canMutate && (
                      <div className="flex gap-2">
                        <button
                          type="button"
                          onClick={() => startEdit(d)}
                          className="text-sm font-medium text-slate-600 hover:text-slate-900"
                        >
                          Edit
                        </button>
                        <button
                          type="button"
                          onClick={() => setDeleteId(d.id)}
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
        )}
      </div>

      {deleteId && (
        <div
          className="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4"
          role="dialog"
          aria-modal="true"
        >
          <div className="max-w-md rounded-xl bg-white p-6 shadow-lg">
            <p className="text-sm text-slate-800">Delete this destination?</p>
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
        <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
          <h2 className="text-lg font-semibold text-slate-900">New destination</h2>
          <form className="mt-4 space-y-4" onSubmit={onCreate}>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="srv">
                Server
              </label>
              <select
                id="srv"
                value={serverId}
                onChange={(e) => setServerId(e.target.value)}
                className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                required
              >
                <option value="">Select server…</option>
                {servers.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name} ({s.host})
                  </option>
                ))}
              </select>
            </div>
            <div className="flex flex-wrap gap-3">
              <div className="min-w-[160px] flex-1">
                <label className="block text-sm font-medium text-slate-700" htmlFor="dname">
                  Name
                </label>
                <input
                  id="dname"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                  required
                />
              </div>
              <div className="min-w-[160px] flex-1">
                <label className="block text-sm font-medium text-slate-700" htmlFor="dslg">
                  Slug
                </label>
                <input
                  id="dslg"
                  value={slug}
                  onChange={(e) => setSlug(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                  required
                />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="ddesc">
                Description (optional)
              </label>
              <input
                id="ddesc"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
              />
            </div>
            <button
              type="submit"
              disabled={pending}
              className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700 disabled:opacity-60"
            >
              {pending ? "Creating…" : "Create"}
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
