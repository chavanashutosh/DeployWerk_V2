import { useCallback, useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { ScrollText } from "lucide-react";
import { apiFetch, type DeployJob } from "@/api";
import { InlineError, LoadingBlock, PageHeader } from "@/components/ui";

export function LogsPage() {
  const { teamId = "" } = useParams();
  const [searchParams] = useSearchParams();
  const jobFromQuery = searchParams.get("job") ?? "";
  const [jobId, setJobId] = useState(jobFromQuery);
  const [job, setJob] = useState<DeployJob | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setJobId(jobFromQuery);
  }, [jobFromQuery]);

  const fetchJob = useCallback(
    async (id: string, opts?: { silent?: boolean }) => {
      if (!teamId || !id.trim()) return null;
      const silent = opts?.silent ?? false;
      if (!silent) {
        setLoading(true);
        setErr(null);
      }
      try {
        const j = await apiFetch<DeployJob>(`/api/v1/teams/${teamId}/deploy-jobs/${id.trim()}`);
        setJob(j);
        setErr(null);
        return j;
      } catch (e) {
        if (!silent) {
          setErr(e instanceof Error ? e.message : "Failed to load job");
          setJob(null);
        }
        return null;
      } finally {
        if (!silent) setLoading(false);
      }
    },
    [teamId],
  );

  useEffect(() => {
    if (!teamId || !jobId.trim()) {
      setJob(null);
      return;
    }
    void fetchJob(jobId.trim());
  }, [teamId, jobId, fetchJob]);

  useEffect(() => {
    if (!teamId || !jobId.trim() || !job) return;
    if (job.status !== "queued" && job.status !== "running") return;
    const id = jobId.trim();
    const t = window.setInterval(() => {
      void fetchJob(id, { silent: true });
    }, 2000);
    return () => window.clearInterval(t);
  }, [teamId, jobId, job, fetchJob]);

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<ScrollText className="h-6 w-6" strokeWidth={1.75} />}
        title="Deploy logs"
        description={
          <>
            Paste a deploy job UUID or open one from{" "}
            <Link to={`/app/teams/${teamId}/deployments`} className="font-medium text-brand-600 hover:text-brand-700">
              Deployments
            </Link>
            . Logs refresh every 2s while the job is queued or running.
          </>
        }
      />

      <div className="dw-card flex flex-wrap items-end gap-3 p-4 sm:p-5">
        <div className="min-w-[280px] flex-1">
          <label className="block text-xs font-medium text-slate-500" htmlFor="jobid">
            Job ID
          </label>
          <input
            id="jobid"
            value={jobId}
            onChange={(e) => setJobId(e.target.value)}
            placeholder="uuid"
            className="dw-input mt-1 font-mono text-sm"
          />
        </div>
        <button
          type="button"
          onClick={() => {
            const id = jobId.trim();
            if (!id || !teamId) return;
            void fetchJob(id);
          }}
          className="dw-btn-primary"
        >
          Load log
        </button>
      </div>
      {loading && !job && <LoadingBlock label="Loading job log…" />}
      <InlineError message={err} />
      {job && (
        <div className="overflow-hidden rounded-xl border border-slate-800 bg-slate-950 shadow-lg ring-1 ring-slate-800/80">
          <div className="border-b border-slate-800 px-4 py-2.5 text-xs text-slate-400">
            <span>
              Status: <span className="font-semibold text-slate-200">{job.status}</span>
            </span>
            <span className="ml-3 font-mono text-slate-500">{job.id}</span>
          </div>
          <pre className="max-h-[min(70vh,32rem)] overflow-auto whitespace-pre-wrap bg-slate-950 p-4 font-mono text-xs leading-relaxed text-slate-100">
            {job.log || "(empty log)"}
          </pre>
        </div>
      )}
    </div>
  );
}
