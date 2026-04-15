import { useEffect, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { apiFetch, type Team } from "@/api";
import { InlineError, LoadingBlock, PageHeader } from "@/components/ui";
import { Box } from "lucide-react";

type DockerContainerRow = {
  id: string;
  name: string;
  status: string;
  image: string;
};

export function ServerDockerPage() {
  const { teamId = "", serverId = "" } = useParams();
  const [teams, setTeams] = useState<Team[]>([]);
  const [rows, setRows] = useState<DockerContainerRow[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [inspect, setInspect] = useState<string | null>(null);
  const [logs, setLogs] = useState<{ ref: string; text: string } | null>(null);
  const [execRef, setExecRef] = useState<string | null>(null);
  const [execArgv, setExecArgv] = useState("ls -la /");

  const team = teams.find((t) => t.id === teamId);
  const canMutate =
    team?.role === "admin" ||
    team?.role === "owner" ||
    !!team?.access_via_organization_admin;

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
    if (!teamId || !serverId) return;
    setErr(null);
    try {
      const list = await apiFetch<DockerContainerRow[]>(
        `/api/v1/teams/${teamId}/servers/${serverId}/docker/containers`,
      );
      setRows(list);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Failed to load containers");
      setRows(null);
    }
  }

  useEffect(() => {
    void load();
  }, [teamId, serverId]);

  async function onInspect(containerRef: string) {
    if (!teamId || !serverId) return;
    setBusy(`insp-${containerRef}`);
    setErr(null);
    try {
      const j = await apiFetch<unknown>(
        `/api/v1/teams/${teamId}/servers/${serverId}/docker/containers/${encodeURIComponent(containerRef)}/inspect`,
      );
      setInspect(JSON.stringify(j, null, 2));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Inspect failed");
    } finally {
      setBusy(null);
    }
  }

  async function onLogs(containerRef: string) {
    if (!teamId || !serverId) return;
    setBusy(`log-${containerRef}`);
    setErr(null);
    try {
      const j = await apiFetch<{ log?: string }>(
        `/api/v1/teams/${teamId}/servers/${serverId}/docker/containers/${encodeURIComponent(containerRef)}/logs?tail=400`,
      );
      setLogs({ ref: containerRef, text: j.log ?? "" });
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Logs failed");
    } finally {
      setBusy(null);
    }
  }

  async function onAction(
    containerRef: string,
    action: "start" | "stop" | "restart",
  ) {
    if (!teamId || !serverId) return;
    setBusy(`${action}-${containerRef}`);
    setErr(null);
    try {
      await apiFetch(
        `/api/v1/teams/${teamId}/servers/${serverId}/docker/containers/${encodeURIComponent(containerRef)}/${action}`,
        { method: "POST" },
      );
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : `${action} failed`);
    } finally {
      setBusy(null);
    }
  }

  async function onExec(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !serverId || !execRef) return;
    const parts = execArgv
      .trim()
      .split(/\s+/)
      .filter(Boolean);
    if (parts.length === 0) return;
    setBusy(`exec-${execRef}`);
    setErr(null);
    try {
      const j = await apiFetch<{ output?: string }>(
        `/api/v1/teams/${teamId}/servers/${serverId}/docker/containers/${encodeURIComponent(execRef)}/exec`,
        {
          method: "POST",
          body: JSON.stringify({ argv: parts }),
        },
      );
      setInspect(j.output ?? "");
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Exec failed");
    } finally {
      setBusy(null);
    }
  }

  if (!teamId || !serverId) {
    return <p className="text-slate-600">Missing team or server.</p>;
  }

  return (
    <div className="space-y-8">
      <PageHeader
        icon={<Box className="h-6 w-6" strokeWidth={1.75} />}
        title="Docker containers"
        description={
          <>
            <Link
              to={`/app/teams/${teamId}/servers`}
              className="font-medium text-brand-600 hover:text-brand-700"
            >
              ← Servers
            </Link>
            <span className="mx-2 text-slate-400">·</span>
            <span className="text-slate-600">Server {serverId.slice(0, 8)}…</span>
          </>
        }
      />
      <p className="text-sm text-amber-800">
        Actions run over SSH as the configured user. Exec is limited to explicit arguments (no shell
        metacharacters).
      </p>
      <InlineError message={err} />
      {rows === null && !err && <LoadingBlock label="Loading containers…" />}
      {rows && rows.length === 0 && <p className="text-sm text-slate-600">No containers reported.</p>}
      {rows && rows.length > 0 && (
        <ul className="divide-y divide-slate-100 rounded-xl border border-slate-200 bg-white">
          {rows.map((r) => (
            <li key={r.id} className="flex flex-wrap items-start justify-between gap-3 p-4">
              <div>
                <p className="font-mono text-sm font-medium text-slate-900">{r.name || r.id}</p>
                <p className="text-xs text-slate-500">{r.status}</p>
                <p className="font-mono text-xs text-slate-600">{r.image}</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  disabled={!!busy}
                  onClick={() => void onInspect(r.id)}
                  className="rounded border border-slate-200 px-2 py-1 text-xs"
                >
                  Inspect
                </button>
                <button
                  type="button"
                  disabled={!!busy}
                  onClick={() => void onLogs(r.id)}
                  className="rounded border border-slate-200 px-2 py-1 text-xs"
                >
                  Logs
                </button>
                {canMutate && (
                  <>
                    <button
                      type="button"
                      disabled={!!busy}
                      onClick={() => void onAction(r.id, "start")}
                      className="rounded border border-emerald-200 bg-emerald-50 px-2 py-1 text-xs text-emerald-900"
                    >
                      Start
                    </button>
                    <button
                      type="button"
                      disabled={!!busy}
                      onClick={() => void onAction(r.id, "stop")}
                      className="rounded border border-amber-200 bg-amber-50 px-2 py-1 text-xs text-amber-900"
                    >
                      Stop
                    </button>
                    <button
                      type="button"
                      disabled={!!busy}
                      onClick={() => void onAction(r.id, "restart")}
                      className="rounded border border-slate-200 px-2 py-1 text-xs"
                    >
                      Restart
                    </button>
                    <button
                      type="button"
                      disabled={!!busy}
                      onClick={() => {
                        setExecRef(r.id);
                        setExecArgv("ls -la /");
                      }}
                      className="rounded border border-violet-200 bg-violet-50 px-2 py-1 text-xs text-violet-900"
                    >
                      Exec…
                    </button>
                  </>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
      {inspect !== null && (
        <div className="rounded-xl border border-slate-200 bg-slate-950 p-4">
          <div className="mb-2 flex justify-between">
            <h3 className="text-sm font-medium text-slate-200">Output / inspect</h3>
            <button
              type="button"
              onClick={() => setInspect(null)}
              className="text-xs text-slate-400 hover:text-white"
            >
              Close
            </button>
          </div>
          <pre className="max-h-[min(60vh,28rem)] overflow-auto whitespace-pre-wrap font-mono text-xs text-slate-100">
            {inspect}
          </pre>
        </div>
      )}
      {logs && (
        <div className="rounded-xl border border-slate-200 bg-slate-950 p-4">
          <div className="mb-2 flex justify-between">
            <h3 className="text-sm font-medium text-slate-200">Logs: {logs.ref}</h3>
            <button
              type="button"
              onClick={() => setLogs(null)}
              className="text-xs text-slate-400 hover:text-white"
            >
              Close
            </button>
          </div>
          <pre className="max-h-[min(50vh,24rem)] overflow-auto whitespace-pre-wrap font-mono text-xs text-slate-100">
            {logs.text}
          </pre>
        </div>
      )}
      {execRef && canMutate && (
        <div className="rounded-xl border border-violet-200 bg-violet-50/50 p-4">
          <h3 className="text-sm font-semibold text-violet-900">Exec in {execRef.slice(0, 12)}…</h3>
          <form className="mt-3 flex flex-wrap items-end gap-2" onSubmit={(e) => void onExec(e)}>
            <div className="min-w-[200px] flex-1">
              <label className="text-xs font-medium text-slate-600" htmlFor="argv">
                Command (space-separated argv)
              </label>
              <input
                id="argv"
                value={execArgv}
                onChange={(e) => setExecArgv(e.target.value)}
                className="mt-1 w-full rounded border border-slate-200 px-2 py-1.5 font-mono text-sm"
              />
            </div>
            <button
              type="submit"
              disabled={!!busy}
              className="rounded bg-violet-600 px-3 py-2 text-sm font-semibold text-white hover:bg-violet-700 disabled:opacity-50"
            >
              Run
            </button>
            <button
              type="button"
              onClick={() => setExecRef(null)}
              className="rounded border border-slate-200 px-3 py-2 text-sm"
            >
              Cancel
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
