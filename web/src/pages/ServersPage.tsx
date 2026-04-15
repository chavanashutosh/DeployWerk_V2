import { useEffect, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import {
  apiFetch,
  type Server,
  type Team,
  type ValidateServerResponse,
} from "@/api";

async function loadServers(teamId: string): Promise<Server[]> {
  return apiFetch<Server[]>(`/api/v1/teams/${teamId}/servers`);
}

export function ServersPage() {
  const { teamId = "" } = useParams();
  const [servers, setServers] = useState<Server[] | null>(null);
  const [teams, setTeams] = useState<Team[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [name, setName] = useState("");
  const [host, setHost] = useState("");
  const [sshPort, setSshPort] = useState("22");
  const [sshUser, setSshUser] = useState("");
  const [privateKey, setPrivateKey] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editHost, setEditHost] = useState("");
  const [editPort, setEditPort] = useState("");
  const [editUser, setEditUser] = useState("");
  const [editKey, setEditKey] = useState("");
  const [editPending, setEditPending] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [validatingId, setValidatingId] = useState<string | null>(null);
  const [validateMsg, setValidateMsg] = useState<string | null>(null);

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
        const list = await loadServers(teamId);
        if (!cancelled) {
          setServers(list);
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load");
          setServers(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !name.trim() || !host.trim() || !sshUser.trim() || !privateKey.trim()) return;
    const port = Number.parseInt(sshPort, 10);
    if (Number.isNaN(port) || port < 1 || port > 65535) {
      setErr("SSH port must be between 1 and 65535");
      return;
    }
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/servers`, {
        method: "POST",
        body: JSON.stringify({
          name: name.trim(),
          host: host.trim(),
          ssh_port: port,
          ssh_user: sshUser.trim(),
          ssh_private_key_pem: privateKey,
        }),
      });
      setName("");
      setHost("");
      setSshPort("22");
      setSshUser("");
      setPrivateKey("");
      setServers(await loadServers(teamId));
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Create failed");
    } finally {
      setPending(false);
    }
  }

  function startEdit(s: Server) {
    setEditingId(s.id);
    setEditName(s.name);
    setEditHost(s.host);
    setEditPort(String(s.ssh_port));
    setEditUser(s.ssh_user);
    setEditKey("");
  }

  async function onSaveEdit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !editingId || !editName.trim() || !editHost.trim() || !editUser.trim()) return;
    const port = Number.parseInt(editPort, 10);
    if (Number.isNaN(port) || port < 1 || port > 65535) {
      setErr("SSH port must be between 1 and 65535");
      return;
    }
    setEditPending(true);
    setErr(null);
    try {
      const body: Record<string, unknown> = {
        name: editName.trim(),
        host: editHost.trim(),
        ssh_port: port,
        ssh_user: editUser.trim(),
      };
      if (editKey.trim()) body.ssh_private_key_pem = editKey;
      await apiFetch(`/api/v1/teams/${teamId}/servers/${editingId}`, {
        method: "PATCH",
        body: JSON.stringify(body),
      });
      setEditingId(null);
      setServers(await loadServers(teamId));
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Update failed");
    } finally {
      setEditPending(false);
    }
  }

  async function onConfirmDelete() {
    if (!teamId || !deleteId) return;
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/servers/${deleteId}`, {
        method: "DELETE",
      });
      setDeleteId(null);
      if (editingId === deleteId) setEditingId(null);
      setServers(await loadServers(teamId));
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Delete failed");
    }
  }

  async function onValidate(serverId: string) {
    if (!teamId) return;
    setValidatingId(serverId);
    setValidateMsg(null);
    setErr(null);
    try {
      const res = await apiFetch<ValidateServerResponse>(
        `/api/v1/teams/${teamId}/servers/${serverId}/validate`,
        { method: "POST" },
      );
      setValidateMsg(
        res.ok
          ? `Validation OK: ${res.detail ?? ""}`
          : `Validation failed: ${res.detail ?? "unknown"}`,
      );
      setServers(await loadServers(teamId));
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Validate failed");
    } finally {
      setValidatingId(null);
    }
  }

  if (!teamId) {
    return <p className="text-slate-600">Missing team.</p>;
  }

  return (
    <div className="space-y-8">
      <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
        <h1 className="text-2xl font-bold text-slate-900">Servers</h1>
        <p className="mt-2 text-sm text-slate-600">
          <Link
            to={`/app/teams/${teamId}/projects`}
            className="font-medium text-brand-600 hover:text-brand-700"
          >
            ← Projects
          </Link>
          <span className="mx-2 text-slate-400">·</span>
          Team <span className="font-medium text-slate-900">{team?.name ?? teamId}</span>
        </p>
        <p className="mt-2 text-sm text-amber-800">
          Private keys are encrypted on the server. Use only keys you are allowed to store.
        </p>
        {err && <p className="mt-4 text-sm text-red-600">{err}</p>}
        {validateMsg && (
          <p className="mt-4 text-sm text-slate-700" role="status">
            {validateMsg}
          </p>
        )}
        {servers && servers.length === 0 && (
          <p className="mt-4 text-sm text-slate-600">No servers yet.</p>
        )}
        {servers && servers.length > 0 && (
          <ul className="mt-6 divide-y divide-slate-100">
            {servers.map((s) => (
              <li key={s.id} className="py-4 first:pt-0">
                {editingId === s.id && canMutate ? (
                  <form className="space-y-3" onSubmit={(e) => void onSaveEdit(e)}>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`sn-${s.id}`}>
                        Name
                      </label>
                      <input
                        id={`sn-${s.id}`}
                        value={editName}
                        onChange={(e) => setEditName(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                        required
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`sh-${s.id}`}>
                        Host
                      </label>
                      <input
                        id={`sh-${s.id}`}
                        value={editHost}
                        onChange={(e) => setEditHost(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                        required
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`sp-${s.id}`}>
                        SSH port
                      </label>
                      <input
                        id={`sp-${s.id}`}
                        type="number"
                        min={1}
                        max={65535}
                        value={editPort}
                        onChange={(e) => setEditPort(e.target.value)}
                        className="mt-1 w-28 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                        required
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`su-${s.id}`}>
                        SSH user
                      </label>
                      <input
                        id={`su-${s.id}`}
                        value={editUser}
                        onChange={(e) => setEditUser(e.target.value)}
                        className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                        required
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-slate-500" htmlFor={`sk-${s.id}`}>
                        New private key (optional)
                      </label>
                      <textarea
                        id={`sk-${s.id}`}
                        value={editKey}
                        onChange={(e) => setEditKey(e.target.value)}
                        rows={4}
                        className="mt-1 w-full max-w-xl rounded-lg border border-slate-200 px-3 py-2 font-mono text-xs"
                        placeholder="Leave empty to keep existing key"
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
                        onClick={() => setEditingId(null)}
                        className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm text-slate-700"
                      >
                        Cancel
                      </button>
                    </div>
                  </form>
                ) : (
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <p className="font-medium text-slate-900">{s.name}</p>
                      <p className="text-sm text-slate-600">
                        <span className="font-mono">{s.ssh_user}</span>@<span className="font-mono">{s.host}</span>:
                        {s.ssh_port}
                      </p>
                      <p className="mt-1 text-sm">
                        <span className="rounded bg-slate-100 px-2 py-0.5 font-mono text-xs text-slate-700">
                          {s.status}
                        </span>
                        {s.last_validation_error && (
                          <span className="ml-2 text-xs text-red-600">{s.last_validation_error}</span>
                        )}
                      </p>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <Link
                        to={`/app/teams/${teamId}/servers/${s.id}/docker`}
                        className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-50"
                      >
                        Docker
                      </Link>
                      {canMutate && (
                        <>
                          <button
                            type="button"
                            disabled={validatingId === s.id}
                            onClick={() => void onValidate(s.id)}
                            className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-60"
                          >
                            {validatingId === s.id ? "Validating…" : "Validate"}
                          </button>
                          <button
                            type="button"
                            onClick={() => startEdit(s)}
                            className="text-sm font-medium text-slate-600 hover:text-slate-900"
                          >
                            Edit
                          </button>
                          <button
                            type="button"
                            onClick={() => setDeleteId(s.id)}
                            className="text-sm font-medium text-red-600 hover:text-red-700"
                          >
                            Delete
                          </button>
                        </>
                      )}
                    </div>
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
            <p className="text-sm text-slate-800">Delete this server record? Stored credentials will be removed.</p>
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
          <h2 className="text-lg font-semibold text-slate-900">Add server</h2>
          <form className="mt-4 max-w-xl space-y-4" onSubmit={(e) => void onCreate(e)}>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="srv-name">
                Display name
              </label>
              <input
                id="srv-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                required
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="srv-host">
                Host
              </label>
              <input
                id="srv-host"
                value={host}
                onChange={(e) => setHost(e.target.value)}
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                placeholder="203.0.113.10 or host.example.com"
                required
              />
            </div>
            <div className="flex flex-wrap gap-4">
              <div>
                <label className="block text-sm font-medium text-slate-700" htmlFor="srv-port">
                  SSH port
                </label>
                <input
                  id="srv-port"
                  type="number"
                  min={1}
                  max={65535}
                  value={sshPort}
                  onChange={(e) => setSshPort(e.target.value)}
                  className="mt-1 w-28 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                  required
                />
              </div>
              <div className="min-w-[200px] flex-1">
                <label className="block text-sm font-medium text-slate-700" htmlFor="srv-user">
                  SSH user
                </label>
                <input
                  id="srv-user"
                  value={sshUser}
                  onChange={(e) => setSshUser(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                  required
                />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-700" htmlFor="srv-key">
                Private key (PEM / OpenSSH)
              </label>
              <textarea
                id="srv-key"
                value={privateKey}
                onChange={(e) => setPrivateKey(e.target.value)}
                rows={6}
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-xs"
                required
              />
            </div>
            <button
              type="submit"
              disabled={pending}
              className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700 disabled:opacity-60"
            >
              {pending ? "Saving…" : "Add server"}
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
