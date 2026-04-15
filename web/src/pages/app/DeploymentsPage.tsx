import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { Inbox, Rocket } from "lucide-react";
import { apiFetch, type Team, type TeamDeploymentRow } from "@/api";
import { EmptyState, InlineError, LoadingBlock, PageHeader } from "@/components/ui";

const statusColor: Record<string, string> = {
  pending_approval: "bg-violet-100 text-violet-800",
  queued: "bg-amber-100 text-amber-800",
  running: "bg-sky-100 text-sky-800",
  succeeded: "bg-emerald-100 text-emerald-800",
  failed: "bg-red-100 text-red-800",
};

export function DeploymentsPage() {
  const { teamId = "" } = useParams();
  const [rows, setRows] = useState<TeamDeploymentRow[] | null>(null);
  const [teams, setTeams] = useState<Team[]>([]);
  const [err, setErr] = useState<string | null>(null);

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
    let interval: ReturnType<typeof setInterval> | undefined;

    async function tick() {
      try {
        const list = await apiFetch<TeamDeploymentRow[]>(
          `/api/v1/teams/${teamId}/deployments?limit=50`,
        );
        if (cancelled) return;
        setRows(list);
        setErr(null);
        const active = list.some(
          (r) =>
            r.status === "queued" ||
            r.status === "running" ||
            r.status === "pending_approval",
        );
        if (active) {
          if (!interval) {
            interval = setInterval(() => void tick(), 3500);
          }
        } else if (interval) {
          clearInterval(interval);
          interval = undefined;
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load");
          setRows(null);
        }
      }
    }

    void tick();
    return () => {
      cancelled = true;
      if (interval) clearInterval(interval);
    };
  }, [teamId]);

  const team = teams.find((t) => t.id === teamId);
  const canMutate = team?.role === "admin" || team?.role === "owner";

  async function approveJob(jobId: string) {
    if (!teamId) return;
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/deploy-jobs/${jobId}/approve`, {
        method: "POST",
      });
      const list = await apiFetch<TeamDeploymentRow[]>(
        `/api/v1/teams/${teamId}/deployments?limit=50`,
      );
      setRows(list);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Approve failed");
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Rocket className="h-6 w-6" strokeWidth={1.75} />}
        title="Deployments"
        description={
          <>
            Jobs across all applications in{" "}
            <span className="font-medium text-slate-800">{team?.name ?? "this team"}</span>. Full log output lives under{" "}
            <Link to={`/app/teams/${teamId}/logs`} className="font-medium text-brand-600 hover:text-brand-700">
              Logs
            </Link>
            .
          </>
        }
      />

      <InlineError message={err} />

      {rows && rows.length > 0 && (
        <div className="flex flex-wrap gap-3 rounded-lg border border-slate-200 bg-slate-50/80 px-4 py-3 text-xs text-slate-600">
          <span className="font-semibold text-slate-700">Status:</span>
          {Object.entries(statusColor).map(([k, cls]) => (
            <span key={k} className="inline-flex items-center gap-1.5">
              <span className={`inline-flex rounded-full px-2 py-0.5 font-medium ${cls}`}>{k}</span>
            </span>
          ))}
        </div>
      )}

      {rows === null && !err && <LoadingBlock label="Loading deployments…" />}

      {rows && rows.length === 0 && (
        <EmptyState
          icon={Inbox}
          title="No deployments yet"
          action={
            <Link
              to={`/app/teams/${teamId}/projects`}
              className="dw-btn-primary inline-flex px-5 py-2.5 text-sm"
            >
              Go to projects
            </Link>
          }
        >
          Create an application under a project environment, then trigger a deploy from the app page or via Git webhook.
        </EmptyState>
      )}
      {rows && rows.length > 0 && (
        <div className="dw-card overflow-hidden shadow-sm">
          <table className="min-w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-100 bg-slate-50 text-xs font-semibold uppercase tracking-wide text-slate-500">
                <th className="px-4 py-3">Status</th>
                <th className="px-4 py-3">Application</th>
                <th className="px-4 py-3">Environment</th>
                <th className="px-4 py-3">Project</th>
                <th className="px-4 py-3">Started</th>
                <th className="px-4 py-3">Links</th>
                <th className="px-4 py-3" />
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.job_id} className="border-b border-slate-50 last:border-0">
                  <td className="px-4 py-3">
                    <span
                      className={`inline-flex rounded-full px-2 py-0.5 text-xs font-medium ${statusColor[r.status] ?? "bg-slate-100 text-slate-700"}`}
                    >
                      {r.status}
                    </span>
                  </td>
                  <td className="px-4 py-3 font-medium text-slate-900">{r.application_name}</td>
                  <td className="px-4 py-3 text-slate-600">{r.environment_name}</td>
                  <td className="px-4 py-3 text-slate-600">{r.project_name}</td>
                  <td className="px-4 py-3 font-mono text-xs text-slate-500">
                    {new Date(r.created_at).toLocaleString()}
                  </td>
                  <td className="px-4 py-3">
                    <div className="flex flex-col gap-1 text-sm">
                      {r.job_kind && (
                        <span className="text-xs font-medium text-violet-700">
                          {r.job_kind}
                          {r.pr_number != null ? ` #${r.pr_number}` : ""}
                        </span>
                      )}
                      {r.deploy_strategy && r.deploy_strategy !== "standard" && (
                        <span
                          className="text-xs text-amber-800"
                          title="Worker still runs a standard single-container replace; strategy is recorded for policy and logs."
                        >
                          strategy: {r.deploy_strategy}
                        </span>
                      )}
                      {r.status === "succeeded" && r.primary_url && (
                        <a
                          href={r.primary_url}
                          target="_blank"
                          rel="noreferrer"
                          className="font-medium text-emerald-700 hover:text-emerald-800"
                        >
                          Open app
                        </a>
                      )}
                      {r.source_compare_url && (
                        <a
                          href={r.source_compare_url}
                          target="_blank"
                          rel="noreferrer"
                          className="font-medium text-violet-700 hover:text-violet-800"
                        >
                          Compare PR
                        </a>
                      )}
                      {r.source_commit_url && (
                        <a
                          href={r.source_commit_url}
                          target="_blank"
                          rel="noreferrer"
                          className="font-medium text-slate-600 hover:text-slate-900"
                        >
                          View commit
                        </a>
                      )}
                      {!r.source_commit_url && r.git_sha && (
                        <span className="font-mono text-xs text-slate-400" title={r.git_sha}>
                          {r.git_sha.slice(0, 7)}
                        </span>
                      )}
                    </div>
                  </td>
                  <td className="px-4 py-3 text-right">
                    <div className="flex flex-col items-end gap-1">
                      {r.status === "pending_approval" && canMutate && (
                        <button
                          type="button"
                          onClick={() => void approveJob(r.job_id)}
                          className="dw-btn-secondary border-violet-200 px-3 py-1.5 text-sm font-semibold text-violet-800 hover:bg-violet-50"
                        >
                          Approve deploy
                        </button>
                      )}
                      {r.status === "pending_approval" && !canMutate && (
                        <span className="text-xs text-slate-500" title="Ask a team owner or admin to approve">
                          Awaiting approval
                        </span>
                      )}
                      <Link
                        to={`/app/teams/${teamId}/logs?job=${r.job_id}`}
                        className="text-sm font-medium text-brand-600 hover:text-brand-700"
                      >
                        View log
                      </Link>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
