import { useEffect, useState } from "react";
import { Copy, Gauge, LineChart } from "lucide-react";
import { apiFetch } from "@/api";
import { InlineError, PageHeader } from "@/components/ui";
import { Panel, useTeamId } from "./_shared";

function RumIngestPanel({ teamId }: { teamId: string }) {
  const [cfg, setCfg] = useState<{ ingest_secret: string } | null>(null);
  const [rumErr, setRumErr] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    if (!teamId) return;
    let c = false;
    (async () => {
      try {
        const r = await apiFetch<{ ingest_secret: string }>(`/api/v1/teams/${teamId}/rum/config`);
        if (!c) {
          setCfg(r);
          setRumErr(null);
        }
      } catch (e) {
        if (!c) {
          setCfg(null);
          setRumErr(e instanceof Error ? e.message : "Could not load RUM config (owners/admins only).");
        }
      }
    })();
    return () => {
      c = true;
    };
  }, [teamId]);

  const origin =
    typeof window !== "undefined" ? `${window.location.protocol}//${window.location.host}` : "";
  const ingestUrl = origin ? `${origin}/api/v1/rum/ingest` : "/api/v1/rum/ingest";

  async function copyText(label: string, text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label);
      setTimeout(() => setCopied(null), 2000);
    } catch {
      setCopied(null);
    }
  }

  const curlExample =
    cfg &&
    `curl -sS -X POST '${ingestUrl}' \\\n  -H 'Authorization: Bearer ${cfg.ingest_secret}' \\\n  -H 'Content-Type: application/json' \\\n  -d '{"page_path":"/","metric_name":"LCP","metric_value":1.2}'`;

  return (
    <Panel title="RUM ingest (real user metrics)">
      {rumErr && <p className="text-sm text-red-600">{rumErr}</p>}
      {cfg && (
        <div className="space-y-3 text-sm">
          <p className="text-slate-600">
            Send browser metrics to the API with your team ingest secret. The secret is created automatically the first
            time an admin opens this panel.
          </p>
          <div>
            <span className="text-xs font-medium text-slate-500">Ingest URL</span>
            <div className="mt-1 flex flex-wrap items-center gap-2">
              <code className="break-all rounded bg-slate-100 px-2 py-1 font-mono text-xs">{ingestUrl}</code>
              <button
                type="button"
                className="inline-flex items-center gap-1 rounded border border-slate-200 px-2 py-1 text-xs hover:bg-slate-50"
                onClick={() => void copyText("url", ingestUrl)}
              >
                <Copy className="h-3.5 w-3.5" strokeWidth={1.75} />
                {copied === "url" ? "Copied" : "Copy"}
              </button>
            </div>
          </div>
          <div>
            <span className="text-xs font-medium text-slate-500">Ingest secret (Bearer token)</span>
            <div className="mt-1 flex flex-wrap items-center gap-2">
              <code className="break-all rounded bg-amber-50 px-2 py-1 font-mono text-xs text-amber-950">
                {cfg.ingest_secret}
              </code>
              <button
                type="button"
                className="inline-flex items-center gap-1 rounded border border-slate-200 px-2 py-1 text-xs hover:bg-slate-50"
                onClick={() => void copyText("secret", cfg.ingest_secret)}
              >
                <Copy className="h-3.5 w-3.5" strokeWidth={1.75} />
                {copied === "secret" ? "Copied" : "Copy"}
              </button>
            </div>
          </div>
          {curlExample && (
            <div>
              <span className="text-xs font-medium text-slate-500">Example</span>
              <pre className="mt-1 max-h-40 overflow-auto rounded-lg bg-slate-900 p-3 font-mono text-xs text-slate-100">
                {curlExample}
              </pre>
              <button
                type="button"
                className="mt-2 inline-flex items-center gap-1 rounded border border-slate-200 px-2 py-1 text-xs hover:bg-slate-50"
                onClick={() => void copyText("curl", curlExample.replace(/\n\\\s*/g, " "))}
              >
                <Copy className="h-3.5 w-3.5" strokeWidth={1.75} />
                {copied === "curl" ? "Copied" : "Copy one-line curl"}
              </button>
            </div>
          )}
        </div>
      )}
    </Panel>
  );
}

export function AnalyticsPageApp() {
  const teamId = useTeamId();
  const [usage, setUsage] = useState<{
    period_days: number;
    deploy_job_count: number;
    succeeded: number;
    failed: number;
  } | null>(null);
  const [rum, setRum] = useState<{
    period_days: number;
    by_metric: [string, number][];
    sample_count: number;
  } | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    if (!teamId) return;
    let c = false;
    (async () => {
      try {
        const [u, r] = await Promise.all([
          apiFetch(`/api/v1/teams/${teamId}/usage?days=30`),
          apiFetch(`/api/v1/teams/${teamId}/rum/summary`),
        ]);
        if (!c) {
          setUsage(u as typeof usage);
          setRum(r as typeof rum);
          setErr(null);
        }
      } catch (e) {
        if (!c) setErr(e instanceof Error ? e.message : "Failed");
      }
    })();
    return () => {
      c = true;
    };
  }, [teamId]);

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<LineChart className="h-6 w-6" strokeWidth={1.75} />}
        title="Analytics"
        description="Deployment activity and RUM aggregates for this team."
      />
      <InlineError message={err} />
      <div className="grid gap-4 md:grid-cols-2">
        <Panel title="Deploy jobs (30 days)">
          {usage ? (
            <dl className="grid grid-cols-2 gap-2 text-sm">
              <dt className="text-slate-500">Total</dt>
              <dd className="font-medium">{usage.deploy_job_count}</dd>
              <dt className="text-slate-500">Succeeded</dt>
              <dd className="text-emerald-700">{usage.succeeded}</dd>
              <dt className="text-slate-500">Failed</dt>
              <dd className="text-red-700">{usage.failed}</dd>
            </dl>
          ) : (
            <p className="text-sm text-slate-500">Loading…</p>
          )}
        </Panel>
        <Panel title="RUM samples (7 days)">
          {rum ? (
            <>
              <p className="text-sm text-slate-600">Events: {rum.sample_count}</p>
              <ul className="mt-2 space-y-1 text-sm">
                {rum.by_metric.map(([k, v]) => (
                  <li key={k} className="flex justify-between gap-2">
                    <span className="font-mono text-slate-700">{k}</span>
                    <span className="text-slate-500">{v.toFixed(2)} avg</span>
                  </li>
                ))}
              </ul>
            </>
          ) : (
            <p className="text-sm text-slate-500">Loading…</p>
          )}
        </Panel>
      </div>
      <RumIngestPanel teamId={teamId} />
    </div>
  );
}

export function SpeedInsightsPageApp() {
  const teamId = useTeamId();
  const [rum, setRum] = useState<{
    by_metric: [string, number][];
    sample_count: number;
  } | null>(null);

  useEffect(() => {
    if (!teamId) return;
    let c = false;
    (async () => {
      try {
        const r = await apiFetch(`/api/v1/teams/${teamId}/rum/summary`);
        if (!c) setRum(r as typeof rum);
      } catch {
        if (!c) setRum(null);
      }
    })();
    return () => {
      c = true;
    };
  }, [teamId]);

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Gauge className="h-6 w-6" strokeWidth={1.75} />}
        title="Speed Insights"
        description="Real user metrics (RUM) ingest and aggregates from your sites and apps."
      />
      <RumIngestPanel teamId={teamId} />
      <Panel title="Real user metrics (RUM)">
        <p className="text-sm text-slate-600">Aggregates below use data from the last 7 days.</p>
        {rum && rum.sample_count === 0 && (
          <p className="mt-3 text-sm text-slate-500">No RUM data yet.</p>
        )}
        {rum && rum.by_metric.length > 0 && (
          <ul className="mt-3 space-y-1 text-sm">
            {rum.by_metric.map(([k, v]) => (
              <li key={k}>
                <span className="font-mono">{k}</span>: {v.toFixed(2)}
              </li>
            ))}
          </ul>
        )}
      </Panel>
    </div>
  );
}
