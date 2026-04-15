import { FormEvent, useEffect, useState } from "react";
import {
  Activity,
  Bot,
  Box,
  Cloud,
  Copy,
  Flag,
  HardDrive,
  LifeBuoy,
  Puzzle,
  Shield,
  Sparkles,
  Wallet,
} from "lucide-react";
import {
  apiFetch,
  apiFetchRaw,
  resolveApiUrl,
  type CostSummaryResponse,
  type RegistryStatusResponse,
} from "@/api";
import { NotificationEndpointsPanel } from "@/components/team/NotificationEndpointsPanel";
import { InlineError, PageHeader } from "@/components/ui";
import { Panel, useTeamId } from "./platform/_shared";

export { AnalyticsPageApp, SpeedInsightsPageApp } from "./platform/analytics-pages";

export function ObservabilityPageApp() {
  const teamId = useTeamId();
  const [sum, setSum] = useState<{
    checks_total: number;
    checks_ok_recent: number;
    last_failures: { check_id: string; error_message?: string | null }[];
  } | null>(null);
  const [checks, setChecks] = useState<
    { id: string; name: string; target_url: string; interval_seconds: number }[]
  >([]);
  const [name, setName] = useState("");
  const [url, setUrl] = useState("https://");
  const [err, setErr] = useState<string | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [edName, setEdName] = useState("");
  const [edUrl, setEdUrl] = useState("");
  const [edInterval, setEdInterval] = useState(60);
  const [otlpCopied, setOtlpCopied] = useState<string | null>(null);
  const [otlpBatches, setOtlpBatches] = useState<
    { id: string; received_at: string; size_bytes: number; content_type: string }[]
  >([]);

  const otlpTracesUrl = teamId ? resolveApiUrl(`/api/v1/teams/${teamId}/otlp/v1/traces`) : "";

  async function copyOtlp(label: string, text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setOtlpCopied(label);
      setTimeout(() => setOtlpCopied(null), 2000);
    } catch {
      setOtlpCopied(null);
    }
  }

  async function load() {
    if (!teamId) return;
    try {
      const [s, c, o] = await Promise.all([
        apiFetch(`/api/v1/teams/${teamId}/observability/summary`),
        apiFetch(`/api/v1/teams/${teamId}/health-checks`),
        apiFetch(`/api/v1/teams/${teamId}/otlp/traces?limit=50`),
      ]);
      setSum(s as typeof sum);
      setChecks(c as typeof checks);
      setOtlpBatches(
        Array.isArray(o)
          ? (o as { id: string; received_at: string; size_bytes: number; content_type: string }[])
          : [],
      );
      setErr(null);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Failed");
    }
  }

  async function downloadOtlpBatch(id: string) {
    if (!teamId) return;
    try {
      const res = await apiFetchRaw(`/api/v1/teams/${teamId}/otlp/traces/${id}`);
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `otlp-${id}.bin`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      setErr("Download failed");
    }
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function addCheck(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/health-checks`, {
      method: "POST",
      body: JSON.stringify({ name, target_url: url, interval_seconds: 60 }),
    });
    setName("");
    setUrl("https://");
    await load();
  }

  function startEdit(c: (typeof checks)[number]) {
    setEditingId(c.id);
    setEdName(c.name);
    setEdUrl(c.target_url);
    setEdInterval(c.interval_seconds);
  }

  async function saveEdit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !editingId) return;
    const iv = Math.min(86_400, Math.max(15, Math.floor(edInterval) || 60));
    await apiFetch(`/api/v1/teams/${teamId}/health-checks/${editingId}`, {
      method: "PATCH",
      body: JSON.stringify({ name: edName, target_url: edUrl, interval_seconds: iv }),
    });
    setEditingId(null);
    await load();
  }

  async function deleteCheck(id: string) {
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/health-checks/${id}`, { method: "DELETE" });
    if (editingId === id) setEditingId(null);
    await load();
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Activity className="h-6 w-6" strokeWidth={1.75} />}
        title="Observability"
        description="Synthetic HTTP checks, OTLP trace ingest (preview), and hooks for future dashboards."
      />
      <InlineError message={err} />
      <Panel title="OTLP traces (preview)">
        <p className="text-sm text-slate-600">
          Send protobuf or JSON OTLP/HTTP to the endpoint below. The API returns <strong>202 Accepted</strong> and stores
          raw batches for short-term retention (default 7 days). A full trace explorer UI is still pending; this is an
          early contract for collectors and agents.
        </p>
        {teamId && (
          <div className="mt-3 space-y-2">
            <span className="text-xs font-medium text-slate-500">POST (OTLP/HTTP)</span>
            <div className="flex flex-wrap items-center gap-2">
              <code className="break-all rounded bg-slate-100 px-2 py-1 font-mono text-xs">{otlpTracesUrl}</code>
              <button
                type="button"
                className="inline-flex items-center gap-1 rounded border border-slate-200 px-2 py-1 text-xs hover:bg-slate-50"
                onClick={() => void copyOtlp("otlp", otlpTracesUrl)}
              >
                <Copy className="h-3.5 w-3.5" strokeWidth={1.75} />
                {otlpCopied === "otlp" ? "Copied" : "Copy"}
              </button>
            </div>
            <p className="text-xs text-slate-500">
              Use your usual OTLP exporter with this URL; authenticate with the same Bearer token as the DeployWerk UI
              API.
            </p>
            {otlpBatches.length > 0 && (
              <div className="mt-4">
                <p className="text-xs font-medium text-slate-500">Recent batches (download raw payload)</p>
                <ul className="mt-2 divide-y divide-slate-100 rounded-lg border border-slate-200 bg-white text-xs">
                  {otlpBatches.map((b) => (
                    <li key={b.id} className="flex flex-wrap items-center justify-between gap-2 px-3 py-2">
                      <span className="font-mono text-slate-700">
                        {new Date(b.received_at).toLocaleString()} · {b.size_bytes} bytes ·{" "}
                        <span className="text-slate-500">{b.content_type || "—"}</span>
                      </span>
                      <button
                        type="button"
                        className="rounded border border-slate-200 px-2 py-0.5 hover:bg-slate-50"
                        onClick={() => void downloadOtlpBatch(b.id)}
                      >
                        Download
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}
      </Panel>
      {sum && (
        <Panel title="Summary">
          <p className="text-sm text-slate-600">
            {sum.checks_total} checks · {sum.checks_ok_recent} OK in the last hour (approx.)
          </p>
          {sum.last_failures.length > 0 && (
            <ul className="mt-2 list-inside list-disc text-sm text-red-700">
              {sum.last_failures.slice(0, 5).map((f) => (
                <li key={f.check_id}>{f.error_message ?? "check failed"}</li>
              ))}
            </ul>
          )}
        </Panel>
      )}
      <Panel title="HTTP health checks">
        <form onSubmit={addCheck} className="flex flex-wrap items-end gap-2">
          <label className="text-sm">
            <span className="text-slate-600">Name</span>
            <input
              className="mt-1 block rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
          </label>
          <label className="min-w-[200px] flex-1 text-sm">
            <span className="text-slate-600">URL</span>
            <input
              className="mt-1 block w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              required
            />
          </label>
          <button
            type="submit"
            className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white"
          >
            Add
          </button>
        </form>
        <ul className="mt-4 divide-y divide-slate-100">
          {checks.map((c) => (
            <li key={c.id} className="py-3 text-sm">
              {editingId === c.id ? (
                <form onSubmit={saveEdit} className="space-y-2 rounded-lg border border-slate-200 bg-slate-50/80 p-3">
                  <div className="flex flex-wrap gap-2">
                    <input
                      className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
                      value={edName}
                      onChange={(e) => setEdName(e.target.value)}
                      required
                    />
                    <input
                      className="min-w-[200px] flex-1 rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                      value={edUrl}
                      onChange={(e) => setEdUrl(e.target.value)}
                      required
                    />
                    <input
                      type="number"
                      min={15}
                      max={86400}
                      className="w-28 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                      value={edInterval}
                      onChange={(e) => setEdInterval(Number(e.target.value))}
                      title="Interval seconds (15–86400)"
                    />
                  </div>
                  <div className="flex gap-2">
                    <button type="submit" className="rounded-lg bg-brand-600 px-3 py-1.5 text-sm text-white">
                      Save
                    </button>
                    <button
                      type="button"
                      className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm"
                      onClick={() => setEditingId(null)}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              ) : (
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <span className="font-medium">{c.name}</span>{" "}
                    <span className="font-mono text-slate-500">{c.target_url}</span>
                    <span className="ml-2 text-xs text-slate-400">every {c.interval_seconds}s</span>
                  </div>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      className="rounded-lg border border-slate-200 px-3 py-1 text-xs font-medium text-slate-700 hover:bg-slate-50"
                      onClick={() => startEdit(c)}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="rounded-lg border border-red-200 px-3 py-1 text-xs font-medium text-red-700 hover:bg-red-50"
                      onClick={() => void deleteCheck(c.id)}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              )}
            </li>
          ))}
        </ul>
      </Panel>
    </div>
  );
}

export function FirewallPageApp() {
  const teamId = useTeamId();
  const [rules, setRules] = useState<
    { id: string; label: string; cidr: string; enabled: boolean }[]
  >([]);
  const [label, setLabel] = useState("");
  const [cidr, setCidr] = useState("");
  const [editingRuleId, setEditingRuleId] = useState<string | null>(null);
  const [edRuleLabel, setEdRuleLabel] = useState("");
  const [edRuleCidr, setEdRuleCidr] = useState("");

  async function load() {
    if (!teamId) return;
    const r = await apiFetch(`/api/v1/teams/${teamId}/firewall-rules`);
    setRules(r as typeof rules);
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function addRule(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/firewall-rules`, {
      method: "POST",
      body: JSON.stringify({ label, cidr }),
    });
    setLabel("");
    setCidr("");
    await load();
  }

  async function toggleRule(r: (typeof rules)[number]) {
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/firewall-rules/${r.id}`, {
      method: "PATCH",
      body: JSON.stringify({ enabled: !r.enabled }),
    });
    await load();
  }

  function startRuleEdit(r: (typeof rules)[number]) {
    setEditingRuleId(r.id);
    setEdRuleLabel(r.label);
    setEdRuleCidr(r.cidr);
  }

  async function saveRuleEdit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !editingRuleId) return;
    await apiFetch(`/api/v1/teams/${teamId}/firewall-rules/${editingRuleId}`, {
      method: "PATCH",
      body: JSON.stringify({ label: edRuleLabel, cidr: edRuleCidr }),
    });
    setEditingRuleId(null);
    await load();
  }

  async function deleteRule(id: string) {
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/firewall-rules/${id}`, { method: "DELETE" });
    if (editingRuleId === id) setEditingRuleId(null);
    await load();
  }

  async function downloadTraefik() {
    if (!teamId) return;
    const res = await apiFetchRaw(`/api/v1/teams/${teamId}/edge/traefik-snippet`);
    const text = await res.text();
    const blob = new Blob([text], { type: "text/yaml" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `deploywerk-traefik-${teamId}.yaml`;
    a.click();
    URL.revokeObjectURL(a.href);
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Shield className="h-6 w-6" strokeWidth={1.75} />}
        title="Firewall"
        description="Team-scoped allowlist rules and Traefik snippets for your own edge—enforcement depends on your gateway config."
      />
      <Panel title="IP allowlist (MVP)">
        <p className="text-sm text-slate-600">
          Rules are stored for your team. Export a Traefik <code className="rounded bg-slate-100 px-1">ipWhiteList</code>{" "}
          snippet to apply on your gateway.
        </p>
        <button
          type="button"
          className="mt-3 rounded-lg border border-slate-200 px-4 py-2 text-sm font-medium text-slate-800 hover:bg-slate-50"
          onClick={() => void downloadTraefik()}
        >
          Download Traefik snippet
        </button>
        <form onSubmit={addRule} className="mt-4 flex flex-wrap gap-2">
          <input
            placeholder="Label"
            className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
          />
          <input
            placeholder="CIDR e.g. 203.0.113.0/24"
            className="min-w-[200px] flex-1 rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
            value={cidr}
            onChange={(e) => setCidr(e.target.value)}
            required
          />
          <button type="submit" className="rounded-lg bg-brand-600 px-4 py-2 text-sm text-white">
            Add rule
          </button>
        </form>
        <ul className="mt-4 space-y-2 text-sm">
          {rules.map((r) => (
            <li key={r.id} className="rounded-lg bg-slate-50 px-3 py-2">
              {editingRuleId === r.id ? (
                <form onSubmit={saveRuleEdit} className="space-y-2">
                  <div className="flex flex-wrap gap-2">
                    <input
                      className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
                      value={edRuleLabel}
                      onChange={(e) => setEdRuleLabel(e.target.value)}
                    />
                    <input
                      className="min-w-[180px] flex-1 rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                      value={edRuleCidr}
                      onChange={(e) => setEdRuleCidr(e.target.value)}
                      required
                    />
                  </div>
                  <div className="flex gap-2">
                    <button type="submit" className="rounded-lg bg-brand-600 px-3 py-1 text-xs text-white">
                      Save
                    </button>
                    <button
                      type="button"
                      className="rounded-lg border border-slate-200 px-3 py-1 text-xs"
                      onClick={() => setEditingRuleId(null)}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              ) : (
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <span>{r.label || "(no label)"}</span>{" "}
                    <span className="font-mono text-slate-600">{r.cidr}</span>
                    <span className={`ml-2 text-xs ${r.enabled ? "text-emerald-600" : "text-slate-400"}`}>
                      {r.enabled ? "enabled" : "disabled"}
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <label className="flex items-center gap-1.5 text-xs text-slate-600">
                      <input
                        type="checkbox"
                        checked={r.enabled}
                        onChange={() => void toggleRule(r)}
                      />
                      On
                    </label>
                    <button
                      type="button"
                      className="rounded-lg border border-slate-200 px-2 py-1 text-xs"
                      onClick={() => startRuleEdit(r)}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="rounded-lg border border-red-200 px-2 py-1 text-xs text-red-700"
                      onClick={() => void deleteRule(r.id)}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              )}
            </li>
          ))}
        </ul>
      </Panel>
    </div>
  );
}

export function CdnPageApp() {
  const teamId = useTeamId();
  const [paths, setPaths] = useState("");
  const [log, setLog] = useState<{ id: string; paths: string; status: string; created_at: string }[]>(
    [],
  );

  async function refresh() {
    if (!teamId) return;
    const r = await apiFetch(`/api/v1/teams/${teamId}/cdn/purge-requests`);
    setLog(r as typeof log);
  }

  useEffect(() => {
    void refresh();
  }, [teamId]);

  async function purge(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/cdn/purge`, {
      method: "POST",
      body: JSON.stringify({ paths }),
    });
    setPaths("");
    await refresh();
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Cloud className="h-6 w-6" strokeWidth={1.75} />}
        title="CDN"
        description="Record purge intent for your edge; apply cache invalidation at your CDN or proxy separately."
      />
      <Panel title="Cache purge (audit log)">
        <p className="text-sm text-slate-600">
          Records purge requests for your team (timestamp, paths, status). Apply changes on your edge separately; this
          control plane keeps an audit trail.
        </p>
        <form onSubmit={purge} className="mt-3 flex gap-2">
          <input
            className="flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
            placeholder="Paths or notes"
            value={paths}
            onChange={(e) => setPaths(e.target.value)}
          />
          <button type="submit" className="rounded-lg bg-brand-600 px-4 py-2 text-sm text-white">
            Record purge
          </button>
        </form>
        <ul className="mt-4 divide-y divide-slate-100 text-sm">
          {log.map((l) => (
            <li key={l.id} className="py-2">
              <span className="font-mono text-xs text-slate-400">{l.created_at}</span> — {l.status}:{" "}
              {l.paths || "—"}
            </li>
          ))}
        </ul>
      </Panel>
    </div>
  );
}

export function IntegrationsPageApp() {
  const teamId = useTeamId();
  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Puzzle className="h-6 w-6" strokeWidth={1.75} />}
        title="Integrations"
        description={
          <>
            Outbound webhooks for deploy lifecycle events. Configure endpoints under{" "}
            <strong>Settings → Notifications</strong>.
          </>
        }
      />
      <NotificationEndpointsPanel teamId={teamId} />
    </div>
  );
}

export function StoragePageApp() {
  const teamId = useTeamId();
  const [rows, setRows] = useState<
    { id: string; name: string; endpoint_url: string; bucket: string; region: string; path_style: boolean }[]
  >([]);
  const [name, setName] = useState("");
  const [endpoint_url, setEndpoint] = useState("");
  const [bucket, setBucket] = useState("");
  const [region, setRegion] = useState("");
  const [path_style, setPathStyle] = useState(true);
  const [access_key, setAk] = useState("");
  const [secret_key, setSk] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [edName, setEdName] = useState("");
  const [edEndpoint, setEdEndpoint] = useState("");
  const [edBucket, setEdBucket] = useState("");
  const [edRegion, setEdRegion] = useState("");
  const [edPathStyle, setEdPathStyle] = useState(true);
  const [edAk, setEdAk] = useState("");
  const [edSk, setEdSk] = useState("");
  const [testNote, setTestNote] = useState<Record<string, string>>({});

  async function load() {
    if (!teamId) return;
    const r = await apiFetch(`/api/v1/teams/${teamId}/storage-backends`);
    setRows(r as typeof rows);
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function create(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/storage-backends`, {
      method: "POST",
      body: JSON.stringify({
        name,
        endpoint_url,
        bucket,
        region: region || "",
        path_style,
        access_key,
        secret_key,
      }),
    });
    setName("");
    setEndpoint("");
    setBucket("");
    setRegion("");
    setPathStyle(true);
    setAk("");
    setSk("");
    await load();
  }

  function startEdit(r: (typeof rows)[number]) {
    setEditingId(r.id);
    setEdName(r.name);
    setEdEndpoint(r.endpoint_url);
    setEdBucket(r.bucket);
    setEdRegion(r.region ?? "");
    setEdPathStyle(r.path_style);
    setEdAk("");
    setEdSk("");
  }

  async function saveEdit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !editingId) return;
    const body: Record<string, unknown> = {
      name: edName,
      endpoint_url: edEndpoint,
      bucket: edBucket,
      region: edRegion,
      path_style: edPathStyle,
    };
    if (edAk.trim()) body.access_key = edAk.trim();
    if (edSk.trim()) body.secret_key = edSk.trim();
    await apiFetch(`/api/v1/teams/${teamId}/storage-backends/${editingId}`, {
      method: "PATCH",
      body: JSON.stringify(body),
    });
    setEditingId(null);
    await load();
  }

  async function deleteRow(id: string) {
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/storage-backends/${id}`, { method: "DELETE" });
    if (editingId === id) setEditingId(null);
    setTestNote((m) => {
      const n = { ...m };
      delete n[id];
      return n;
    });
    await load();
  }

  async function testRow(id: string) {
    if (!teamId) return;
    try {
      const res = await apiFetch<Record<string, unknown>>(`/api/v1/teams/${teamId}/storage-backends/${id}/test`, {
        method: "POST",
      });
      setTestNote((m) => ({ ...m, [id]: JSON.stringify(res, null, 2) }));
    } catch (err) {
      setTestNote((m) => ({
        ...m,
        [id]: err instanceof Error ? err.message : "Test failed",
      }));
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<HardDrive className="h-6 w-6" strokeWidth={1.75} />}
        title="Storage"
        description="S3-compatible backend definitions for team workflows; credentials are stored encrypted server-side."
      />
      <Panel title="S3-compatible backends">
        <form onSubmit={create} className="grid gap-2 md:grid-cols-2">
          <label className="text-sm">
            <span className="text-slate-600">Name</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
          </label>
          <label className="text-sm">
            <span className="text-slate-600">Endpoint URL</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              value={endpoint_url}
              onChange={(e) => setEndpoint(e.target.value)}
              required
            />
          </label>
          <label className="text-sm md:col-span-2">
            <span className="text-slate-600">Bucket</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={bucket}
              onChange={(e) => setBucket(e.target.value)}
              required
            />
          </label>
          <label className="text-sm">
            <span className="text-slate-600">Region</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={region}
              onChange={(e) => setRegion(e.target.value)}
              placeholder="Optional"
            />
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={path_style} onChange={(e) => setPathStyle(e.target.checked)} />
            Path-style addressing
          </label>
          <label className="text-sm">
            <span className="text-slate-600">Access key</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              value={access_key}
              onChange={(e) => setAk(e.target.value)}
              required
            />
          </label>
          <label className="text-sm">
            <span className="text-slate-600">Secret key</span>
            <input
              type="password"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              value={secret_key}
              onChange={(e) => setSk(e.target.value)}
              required
            />
          </label>
          <div className="md:col-span-2">
            <button type="submit" className="rounded-lg bg-brand-600 px-4 py-2 text-sm text-white">
              Add backend
            </button>
          </div>
        </form>
        <ul className="mt-4 space-y-3 text-sm">
          {rows.map((r) => (
            <li key={r.id} className="rounded-lg border border-slate-100 p-3">
              {editingId === r.id ? (
                <form onSubmit={saveEdit} className="space-y-2">
                  <div className="grid gap-2 md:grid-cols-2">
                    <input
                      className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
                      value={edName}
                      onChange={(e) => setEdName(e.target.value)}
                      required
                    />
                    <input
                      className="rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                      value={edEndpoint}
                      onChange={(e) => setEdEndpoint(e.target.value)}
                      required
                    />
                    <input
                      className="md:col-span-2 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                      value={edBucket}
                      onChange={(e) => setEdBucket(e.target.value)}
                      required
                    />
                    <input
                      className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
                      value={edRegion}
                      onChange={(e) => setEdRegion(e.target.value)}
                      placeholder="Region"
                    />
                    <label className="flex items-center gap-2 text-sm">
                      <input type="checkbox" checked={edPathStyle} onChange={(e) => setEdPathStyle(e.target.checked)} />
                      Path-style
                    </label>
                    <input
                      className="rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                      value={edAk}
                      onChange={(e) => setEdAk(e.target.value)}
                      placeholder="New access key (optional)"
                    />
                    <input
                      type="password"
                      className="rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                      value={edSk}
                      onChange={(e) => setEdSk(e.target.value)}
                      placeholder="New secret key (optional)"
                    />
                  </div>
                  <div className="flex gap-2">
                    <button type="submit" className="rounded-lg bg-brand-600 px-3 py-1.5 text-xs text-white">
                      Save
                    </button>
                    <button
                      type="button"
                      className="rounded-lg border border-slate-200 px-3 py-1.5 text-xs"
                      onClick={() => setEditingId(null)}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              ) : (
                <>
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <div>
                      <span className="font-medium">{r.name}</span> — {r.bucket}{" "}
                      <span className="font-mono text-slate-500">{r.endpoint_url}</span>
                      <p className="mt-1 text-xs text-slate-500">
                        region: {r.region || "—"} · path-style: {r.path_style ? "yes" : "no"}
                      </p>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <button
                        type="button"
                        className="rounded-lg border border-slate-200 px-2 py-1 text-xs"
                        onClick={() => void testRow(r.id)}
                      >
                        Test reachability
                      </button>
                      <button
                        type="button"
                        className="rounded-lg border border-slate-200 px-2 py-1 text-xs"
                        onClick={() => startEdit(r)}
                      >
                        Edit
                      </button>
                      <button
                        type="button"
                        className="rounded-lg border border-red-200 px-2 py-1 text-xs text-red-700"
                        onClick={() => void deleteRow(r.id)}
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                  {testNote[r.id] && (
                    <pre className="mt-2 max-h-32 overflow-auto rounded bg-slate-50 p-2 font-mono text-xs text-slate-700">
                      {testNote[r.id]}
                    </pre>
                  )}
                </>
              )}
            </li>
          ))}
        </ul>
      </Panel>
    </div>
  );
}

export function FlagsPageApp() {
  const teamId = useTeamId();
  const [rows, setRows] = useState<
    {
      id: string;
      flag_key: string;
      enabled: boolean;
      value_json: unknown;
      environment_id?: string | null;
    }[]
  >([]);
  const [flag_key, setKey] = useState("");
  const [enabled, setEn] = useState(true);
  const [createEnvId, setCreateEnvId] = useState("");
  const [envOptions, setEnvOptions] = useState<{ id: string; label: string }[]>([]);
  const [valueEditId, setValueEditId] = useState<string | null>(null);
  const [valueEditText, setValueEditText] = useState("");
  const [valueErr, setValueErr] = useState<string | null>(null);

  async function load() {
    if (!teamId) return;
    const r = await apiFetch(`/api/v1/teams/${teamId}/feature-flags`);
    setRows(r as typeof rows);
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  useEffect(() => {
    if (!teamId) return;
    let cancelled = false;
    (async () => {
      try {
        const projects = await apiFetch<{ id: string; name: string }[]>(`/api/v1/teams/${teamId}/projects`);
        const opts: { id: string; label: string }[] = [];
        for (const p of projects) {
          const envs = await apiFetch<{ id: string; name: string }[]>(
            `/api/v1/teams/${teamId}/projects/${p.id}/environments`,
          );
          for (const e of envs) {
            opts.push({ id: e.id, label: `${p.name} / ${e.name}` });
          }
        }
        if (!cancelled) setEnvOptions(opts);
      } catch {
        if (!cancelled) setEnvOptions([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId]);

  async function create(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    const body: Record<string, unknown> = { flag_key, enabled, value_json: {} };
    if (createEnvId.trim()) body.environment_id = createEnvId.trim();
    await apiFetch(`/api/v1/teams/${teamId}/feature-flags`, {
      method: "POST",
      body: JSON.stringify(body),
    });
    setKey("");
    setCreateEnvId("");
    await load();
  }

  async function toggleFlag(r: (typeof rows)[number]) {
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/feature-flags/${r.id}`, {
      method: "PATCH",
      body: JSON.stringify({ enabled: !r.enabled }),
    });
    await load();
  }

  function startValueEdit(r: (typeof rows)[number]) {
    setValueEditId(r.id);
    setValueEditText(JSON.stringify(r.value_json ?? {}, null, 2));
    setValueErr(null);
  }

  async function saveValueJson(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !valueEditId) return;
    let parsed: unknown;
    try {
      parsed = JSON.parse(valueEditText);
    } catch {
      setValueErr("Invalid JSON");
      return;
    }
    setValueErr(null);
    await apiFetch(`/api/v1/teams/${teamId}/feature-flags/${valueEditId}`, {
      method: "PATCH",
      body: JSON.stringify({ value_json: parsed }),
    });
    setValueEditId(null);
    await load();
  }

  async function deleteFlag(id: string) {
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/feature-flags/${id}`, { method: "DELETE" });
    if (valueEditId === id) setValueEditId(null);
    await load();
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Flag className="h-6 w-6" strokeWidth={1.75} />}
        title="Feature flags"
        description="JSON values keyed per environment; consume from your apps via API or SDK patterns you own."
      />
      <Panel title="Team flags">
        <form onSubmit={create} className="flex flex-wrap items-end gap-2">
          <input
            className="rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
            placeholder="flag_key"
            value={flag_key}
            onChange={(e) => setKey(e.target.value)}
            required
          />
          <label className="text-sm">
            <span className="block text-xs text-slate-500">Scope</span>
            <select
              className="mt-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={createEnvId}
              onChange={(e) => setCreateEnvId(e.target.value)}
            >
              <option value="">Team-wide</option>
              {envOptions.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={enabled} onChange={(e) => setEn(e.target.checked)} />
            Enabled
          </label>
          <button type="submit" className="rounded-lg bg-brand-600 px-4 py-2 text-sm text-white">
            Create
          </button>
        </form>
        <ul className="mt-4 space-y-3">
          {rows.map((r) => (
            <li key={r.id} className="rounded-lg bg-slate-50 px-3 py-2 text-sm">
              {valueEditId === r.id ? (
                <form onSubmit={saveValueJson} className="space-y-2">
                  <p className="font-mono text-xs text-slate-600">{r.flag_key}</p>
                  <textarea
                    className="h-28 w-full rounded-lg border border-slate-200 p-2 font-mono text-xs"
                    value={valueEditText}
                    onChange={(e) => setValueEditText(e.target.value)}
                  />
                  {valueErr && <p className="text-xs text-red-600">{valueErr}</p>}
                  <div className="flex gap-2">
                    <button type="submit" className="rounded-lg bg-brand-600 px-3 py-1 text-xs text-white">
                      Save JSON
                    </button>
                    <button
                      type="button"
                      className="rounded-lg border border-slate-200 px-3 py-1 text-xs"
                      onClick={() => setValueEditId(null)}
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              ) : (
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <span className="font-mono font-medium">{r.flag_key}</span>
                    <span className="ml-2 text-xs text-slate-500">
                      {r.environment_id ? `env ${r.environment_id.slice(0, 8)}…` : "team-wide"}
                    </span>
                    <p className="mt-1 font-mono text-xs text-slate-600">
                      {JSON.stringify(r.value_json ?? {})}
                    </p>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <label className="flex items-center gap-1 text-xs text-slate-600">
                      <input type="checkbox" checked={r.enabled} onChange={() => void toggleFlag(r)} />
                      On
                    </label>
                    <button
                      type="button"
                      className="rounded-lg border border-slate-200 px-2 py-1 text-xs"
                      onClick={() => startValueEdit(r)}
                    >
                      Edit JSON
                    </button>
                    <button
                      type="button"
                      className="rounded-lg border border-red-200 px-2 py-1 text-xs text-red-700"
                      onClick={() => void deleteFlag(r.id)}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              )}
            </li>
          ))}
        </ul>
      </Panel>
    </div>
  );
}

export function AgentPageApp() {
  const teamId = useTeamId();
  const [rows, setRows] = useState<
    { id: string; name: string; version?: string | null; last_seen_at?: string | null }[]
  >([]);
  const [name, setName] = useState("");
  const [token, setToken] = useState<string | null>(null);

  async function load() {
    if (!teamId) return;
    const r = await apiFetch(`/api/v1/teams/${teamId}/agents`);
    setRows(r as typeof rows);
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function reg(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    const res = await apiFetch<{ token: string }>(`/api/v1/teams/${teamId}/agents`, {
      method: "POST",
      body: JSON.stringify({ name }),
    });
    setToken(res.token);
    setName("");
    await load();
  }

  async function revokeAgent(id: string, displayName: string) {
    if (!teamId) return;
    if (!window.confirm(`Revoke agent “${displayName}”? Its token will stop working.`)) return;
    await apiFetch(`/api/v1/teams/${teamId}/agents/${id}`, { method: "DELETE" });
    await load();
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Bot className="h-6 w-6" strokeWidth={1.75} />}
        title="Agent"
        description="Register lightweight agents for heartbeats and future host metrics—tokens are shown once at creation."
      />
      <Panel title="Register host agent">
        <p className="text-sm text-slate-600">
          Run the <code className="rounded bg-slate-100 px-1">deploywerk-agent</code> binary with{" "}
          <code className="rounded bg-slate-100 px-1">DEPLOYWERK_AGENT_TOKEN</code> and{" "}
          <code className="rounded bg-slate-100 px-1">DEPLOYWERK_API_URL</code>.
        </p>
        <form onSubmit={reg} className="mt-3 flex gap-2">
          <input
            className="flex-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
            placeholder="Agent name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
          <button type="submit" className="rounded-lg bg-brand-600 px-4 py-2 text-sm text-white">
            Create token
          </button>
        </form>
        {token && (
          <p className="mt-3 break-all rounded-lg bg-amber-50 p-3 font-mono text-xs text-amber-900">
            Save this token now (shown once): {token}
          </p>
        )}
        <ul className="mt-4 space-y-2 text-sm">
          {rows.map((r) => (
            <li key={r.id} className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-slate-100 px-3 py-2">
              <div>
                <span className="font-medium">{r.name}</span> — last seen:{" "}
                <span className="text-slate-600">{r.last_seen_at ?? "never"}</span>{" "}
                <span className="text-slate-400">({r.version ?? "?"})</span>
              </div>
              <button
                type="button"
                className="rounded-lg border border-red-200 px-2 py-1 text-xs text-red-700 hover:bg-red-50"
                onClick={() => void revokeAgent(r.id, r.name)}
              >
                Revoke
              </button>
            </li>
          ))}
        </ul>
      </Panel>
    </div>
  );
}

export function AiGatewayPageApp() {
  const teamId = useTeamId();
  const [routes, setRoutes] = useState<
    { id: string; name: string; path_prefix: string; upstream_url: string; enabled: boolean }[]
  >([]);
  const [name, setName] = useState("");
  const [path_prefix, setPrefix] = useState("/v1/");
  const [upstream_url, setUp] = useState("https://api.openai.com");
  const [routeId, setRouteId] = useState("");
  const [invokeBody, setInvokeBody] = useState('{"model":"gpt-4o-mini","messages":[]}');
  const [invokeResult, setInvokeResult] = useState<string | null>(null);
  const [invokeErr, setInvokeErr] = useState<string | null>(null);

  async function load() {
    if (!teamId) return;
    const r = await apiFetch(`/api/v1/teams/${teamId}/ai-gateway/routes`);
    setRoutes(r as typeof routes);
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function create(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/ai-gateway/routes`, {
      method: "POST",
      body: JSON.stringify({ name, path_prefix, upstream_url }),
    });
    setName("");
    await load();
  }

  async function toggleRoute(r: (typeof routes)[number]) {
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/ai-gateway/routes/${r.id}`, {
      method: "PATCH",
      body: JSON.stringify({ enabled: !r.enabled }),
    });
    await load();
  }

  async function deleteRoute(id: string) {
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/ai-gateway/routes/${id}`, { method: "DELETE" });
    if (routeId === id) setRouteId("");
    await load();
  }

  async function invoke(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !routeId) return;
    let body: unknown;
    try {
      body = JSON.parse(invokeBody);
    } catch {
      setInvokeErr("Invalid JSON body");
      setInvokeResult(null);
      return;
    }
    setInvokeErr(null);
    try {
      const res = await apiFetch<unknown>(`/api/v1/teams/${teamId}/ai-gateway/invoke`, {
        method: "POST",
        body: JSON.stringify({ route_id: routeId, path_suffix: "", body }),
      });
      setInvokeResult(JSON.stringify(res, null, 2).slice(0, 8000));
    } catch (err) {
      setInvokeResult(null);
      setInvokeErr(err instanceof Error ? err.message : "Invoke failed");
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Sparkles className="h-6 w-6" strokeWidth={1.75} />}
        title="AI Gateway"
        description="Server-side JSON proxy routes to upstream LLM APIs—review keys, quotas, and data residency for your org."
      />
      <Panel title="Routes (server-side proxy)">
        <p className="text-sm text-slate-600">
          Proxies JSON POSTs from the API to your upstream. Review keys and data residency before use.
        </p>
        <form onSubmit={create} className="mt-3 grid gap-2 md:grid-cols-2">
          <input
            className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
            placeholder="Name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
          <input
            className="rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
            placeholder="Path prefix"
            value={path_prefix}
            onChange={(e) => setPrefix(e.target.value)}
          />
          <input
            className="md:col-span-2 rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
            placeholder="Upstream base URL"
            value={upstream_url}
            onChange={(e) => setUp(e.target.value)}
            required
          />
          <button type="submit" className="rounded-lg bg-brand-600 px-4 py-2 text-sm text-white">
            Add route
          </button>
        </form>
        <ul className="mt-4 space-y-2 text-sm">
          {routes.map((r) => (
            <li
              key={r.id}
              className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-slate-100 px-3 py-2 font-mono text-xs text-slate-700"
            >
              <div>
                <span className="font-sans font-medium text-slate-900">{r.name}</span>{" "}
                <span className="text-slate-500">{r.path_prefix}</span> → {r.upstream_url}
                <span className={`ml-2 font-sans text-[11px] ${r.enabled ? "text-emerald-600" : "text-slate-400"}`}>
                  {r.enabled ? "enabled" : "disabled"}
                </span>
              </div>
              <div className="flex items-center gap-2 font-sans">
                <label className="flex items-center gap-1 text-[11px] text-slate-600">
                  <input type="checkbox" checked={r.enabled} onChange={() => void toggleRoute(r)} />
                  On
                </label>
                <button
                  type="button"
                  className="rounded border border-red-200 px-2 py-0.5 text-[11px] text-red-700"
                  onClick={() => void deleteRoute(r.id)}
                >
                  Delete
                </button>
              </div>
            </li>
          ))}
        </ul>
      </Panel>
      <Panel title="Test invoke (admin)">
        <form onSubmit={invoke} className="space-y-2">
          <select
            className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
            value={routeId}
            onChange={(e) => setRouteId(e.target.value)}
            required
          >
            <option value="">Select route</option>
            {routes.map((r) => (
              <option key={r.id} value={r.id}>
                {r.name}
              </option>
            ))}
          </select>
          <textarea
            className="h-32 w-full rounded-lg border border-slate-200 p-3 font-mono text-xs"
            value={invokeBody}
            onChange={(e) => setInvokeBody(e.target.value)}
          />
          <button type="submit" className="rounded-lg border border-slate-300 px-4 py-2 text-sm">
            POST via API proxy
          </button>
          {invokeErr && <p className="text-sm text-red-600">{invokeErr}</p>}
          {invokeResult && (
            <pre className="max-h-64 overflow-auto rounded-lg bg-slate-50 p-3 font-mono text-xs text-slate-800">
              {invokeResult}
            </pre>
          )}
        </form>
      </Panel>
    </div>
  );
}

export function SandboxesPageApp() {
  const teamId = useTeamId();
  const origin =
    typeof window !== "undefined" ? `${window.location.protocol}//${window.location.host}` : "";
  const [rows, setRows] = useState<
    {
      id: string;
      branch: string;
      commit_sha: string;
      status: string;
      created_at: string;
      meta?: { deploy_job_ids?: string[]; repository?: string };
    }[]
  >([]);
  const [hookPath, setHookPath] = useState("");
  const [secretConfigured, setSecretConfigured] = useState(false);
  const [hookSecret, setHookSecret] = useState("");
  const [hookErr, setHookErr] = useState<string | null>(null);
  const [glPath, setGlPath] = useState("");
  const [glSecCfg, setGlSecCfg] = useState(false);
  const [glSecret, setGlSecret] = useState("");
  const [glErr, setGlErr] = useState<string | null>(null);
  const [ghInstId, setGhInstId] = useState("");
  const [ghLogin, setGhLogin] = useState("");
  const [ghInstMsg, setGhInstMsg] = useState<string | null>(null);
  const [ghInstErr, setGhInstErr] = useState<string | null>(null);
  const [ghInstRows, setGhInstRows] = useState<
    { id: string; installation_id: number; account_login?: string | null; created_at: string }[]
  >([]);
  const [ghInstallUrl, setGhInstallUrl] = useState<string | null>(null);
  const [ghInstallUrlHint, setGhInstallUrlHint] = useState<string | null>(null);

  async function load() {
    if (!teamId) return;
    const r = await apiFetch(`/api/v1/teams/${teamId}/preview-deployments`);
    setRows(r as typeof rows);
  }

  async function loadHookConfig() {
    if (!teamId) return;
    try {
      const c = await apiFetch<{ hook_path: string; secret_configured: boolean }>(
        `/api/v1/teams/${teamId}/github-hook-config`,
      );
      setHookPath(c.hook_path);
      setSecretConfigured(c.secret_configured);
      setHookErr(null);
    } catch {
      setHookPath(`/api/v1/hooks/github/${teamId}`);
    }
  }

  async function loadGitlabHookConfig() {
    if (!teamId) return;
    try {
      const c = await apiFetch<{ hook_path: string; secret_configured: boolean }>(
        `/api/v1/teams/${teamId}/gitlab-hook-config`,
      );
      setGlPath(c.hook_path);
      setGlSecCfg(c.secret_configured);
      setGlErr(null);
    } catch {
      setGlPath(`/api/v1/hooks/gitlab/${teamId}`);
    }
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  useEffect(() => {
    void loadHookConfig();
  }, [teamId]);

  useEffect(() => {
    void loadGitlabHookConfig();
  }, [teamId]);

  async function loadGhInstallations() {
    if (!teamId) return;
    try {
      const r = await apiFetch<
        { id: string; installation_id: number; account_login?: string | null; created_at: string }[]
      >(`/api/v1/teams/${teamId}/github-app/installations`);
      setGhInstRows(r);
    } catch {
      setGhInstRows([]);
    }
  }

  async function loadGhInstallUrl() {
    if (!teamId) return;
    setGhInstallUrlHint(null);
    try {
      const r = await apiFetch<{ url: string }>(`/api/v1/teams/${teamId}/github-app/install-url`);
      setGhInstallUrl(r.url);
    } catch {
      setGhInstallUrl(null);
      setGhInstallUrlHint(
        "Install link requires GITHUB_APP_SLUG on the API. You can still open your app’s page on GitHub manually.",
      );
    }
  }

  useEffect(() => {
    void loadGhInstallations();
    void loadGhInstallUrl();
  }, [teamId]);

  async function saveHookSecret(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    try {
      await apiFetch(`/api/v1/teams/${teamId}/github-hook-config`, {
        method: "PUT",
        body: JSON.stringify({ secret: hookSecret.trim() || null }),
      });
      setHookSecret("");
      await loadHookConfig();
    } catch (e) {
      setHookErr(e instanceof Error ? e.message : "Failed to save secret");
    }
  }

  async function saveGlSecret(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    try {
      await apiFetch(`/api/v1/teams/${teamId}/gitlab-hook-config`, {
        method: "PUT",
        body: JSON.stringify({ secret: glSecret.trim() || null }),
      });
      setGlSecret("");
      await loadGitlabHookConfig();
    } catch (e) {
      setGlErr(e instanceof Error ? e.message : "Failed to save secret");
    }
  }

  async function registerGhInst(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !ghInstId.trim()) return;
    setGhInstErr(null);
    setGhInstMsg(null);
    try {
      const id = Number(ghInstId.trim());
      if (!Number.isFinite(id) || id <= 0) throw new Error("Invalid installation id");
      await apiFetch(`/api/v1/teams/${teamId}/github-app/installation`, {
        method: "POST",
        body: JSON.stringify({
          installation_id: id,
          account_login: ghLogin.trim() || undefined,
        }),
      });
      setGhInstMsg("Installation registered for this team.");
      setGhInstId("");
      setGhLogin("");
      await loadGhInstallations();
    } catch (e) {
      setGhInstErr(e instanceof Error ? e.message : "Failed");
    }
  }

  const fullHookUrl = origin ? `${origin}${hookPath || `/api/v1/hooks/github/${teamId}`}` : hookPath;
  const fullGlUrl = origin ? `${origin}${glPath || `/api/v1/hooks/gitlab/${teamId}`}` : glPath;
  const fullGhAppUrl = origin ? `${origin}/api/v1/hooks/github-app` : "/api/v1/hooks/github-app";

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Box className="h-6 w-6" strokeWidth={1.75} />}
        title="Sandboxes & Git deploy"
        description="Wire GitHub, GitLab, and GitHub App webhooks so pushes and PRs flow into deploy jobs."
      />
      <Panel title="GitHub webhook">
        <p className="text-sm text-slate-600">
          Add a <strong>push</strong> webhook in your GitHub repo pointing to the URL below. Content type{" "}
          <code className="rounded bg-slate-100 px-1">application/json</code>. When a secret is set here, GitHub
          must send <code className="rounded bg-slate-100 px-1">X-Hub-Signature-256</code> (standard GitHub signing).
        </p>
        <p className="mt-3 break-all font-mono text-sm text-slate-800">{fullHookUrl}</p>
        <p className="mt-2 text-xs text-slate-500">
          Secret configured: {secretConfigured ? "yes" : "no"} (recommended for production)
        </p>
        {hookErr && <p className="mt-2 text-sm text-red-600">{hookErr}</p>}
        <form onSubmit={saveHookSecret} className="mt-4 flex flex-wrap items-end gap-2">
          <label className="min-w-[240px] flex-1 text-sm">
            <span className="text-slate-600">Webhook secret</span>
            <input
              type="password"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              value={hookSecret}
              onChange={(e) => setHookSecret(e.target.value)}
              placeholder="Paste GitHub webhook secret"
              autoComplete="off"
            />
          </label>
          <button
            type="submit"
            className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white"
          >
            Save secret
          </button>
        </form>
        <p className="mt-4 text-sm text-slate-600">
          Map branches to deploys in <strong>Applications</strong>: enable auto-deploy and set branch pattern per
          app (e.g. one app with <code className="rounded bg-slate-100 px-1">main</code>, another with{" "}
          <code className="rounded bg-slate-100 px-1">*</code> or <code className="rounded bg-slate-100 px-1">develop</code>
          ). Each matching app queues a deploy job using its configured Docker image.
        </p>
      </Panel>
      <Panel title="GitLab push webhook">
        <p className="text-sm text-slate-600">
          In GitLab project <strong>Settings → Webhooks</strong>, add a <strong>push</strong> URL and optional secret.
          GitLab sends <code className="rounded bg-slate-100 px-1">X-Gitlab-Token</code> when a secret is configured.
          Set <code className="rounded bg-slate-100 px-1">git_repo_full_name</code> on each application to the project{" "}
          <code className="rounded bg-slate-100 px-1">path_with_namespace</code> (e.g.{" "}
          <code className="rounded bg-slate-100 px-1">mygroup/myproject</code>).
        </p>
        <p className="mt-3 break-all font-mono text-sm text-slate-800">{fullGlUrl}</p>
        <p className="mt-2 text-xs text-slate-500">Secret configured: {glSecCfg ? "yes" : "no"}</p>
        {glErr && <p className="mt-2 text-sm text-red-600">{glErr}</p>}
        <form onSubmit={saveGlSecret} className="mt-4 flex flex-wrap items-end gap-2">
          <label className="min-w-[240px] flex-1 text-sm">
            <span className="text-slate-600">Webhook secret (GitLab token)</span>
            <input
              type="password"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              value={glSecret}
              onChange={(e) => setGlSecret(e.target.value)}
              placeholder="Matches X-Gitlab-Token"
              autoComplete="off"
            />
          </label>
          <button type="submit" className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white">
            Save secret
          </button>
        </form>
      </Panel>
      <Panel title="GitHub App (PR previews)">
        <p className="text-sm text-slate-600">
          Configure a GitHub App with webhook URL <span className="break-all font-mono">{fullGhAppUrl}</span> and set{" "}
          <code className="rounded bg-slate-100 px-1">GITHUB_APP_WEBHOOK_SECRET</code> on the API (same HMAC scheme as
          repository webhooks). If the webhook returns <strong>404</strong>, that secret is missing. Register the
          installation for this team after installing the app. Enable <strong>PR preview deploys</strong> on matching
          applications.
        </p>
        {ghInstallUrl && (
          <p className="mt-3 text-sm">
            <a
              href={ghInstallUrl}
              target="_blank"
              rel="noreferrer"
              className="font-medium text-brand-600 hover:text-brand-700"
            >
              Open GitHub install page
            </a>
            <span className="ml-2 text-slate-500">(from API GITHUB_APP_SLUG)</span>
          </p>
        )}
        {ghInstallUrlHint && <p className="mt-2 text-xs text-slate-500">{ghInstallUrlHint}</p>}
        {ghInstRows.length > 0 && (
          <div className="mt-3 rounded-lg border border-slate-100 bg-slate-50/80 p-3 text-sm">
            <p className="font-medium text-slate-800">Registered installations</p>
            <ul className="mt-2 space-y-1 font-mono text-xs text-slate-700">
              {ghInstRows.map((row) => (
                <li key={row.id}>
                  id {row.installation_id}
                  {row.account_login ? ` — ${row.account_login}` : ""}
                </li>
              ))}
            </ul>
          </div>
        )}
        {ghInstMsg && <p className="mt-2 text-sm text-emerald-700">{ghInstMsg}</p>}
        {ghInstErr && <p className="mt-2 text-sm text-red-600">{ghInstErr}</p>}
        <form onSubmit={registerGhInst} className="mt-4 flex flex-wrap items-end gap-3">
          <label className="text-sm">
            <span className="text-slate-600">Installation ID</span>
            <input
              type="text"
              inputMode="numeric"
              className="mt-1 w-40 rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
              value={ghInstId}
              onChange={(e) => setGhInstId(e.target.value)}
              placeholder="12345678"
            />
          </label>
          <label className="min-w-[200px] flex-1 text-sm">
            <span className="text-slate-600">Account / org (optional)</span>
            <input
              type="text"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={ghLogin}
              onChange={(e) => setGhLogin(e.target.value)}
              placeholder="my-org"
            />
          </label>
          <button type="submit" className="rounded-lg border border-slate-300 px-4 py-2 text-sm">
            Register installation
          </button>
        </form>
      </Panel>
      <Panel title="Recent webhook activity (preview rows)">
        <ul className="space-y-2 text-sm">
          {rows.map((r) => (
            <li key={r.id} className="rounded-lg border border-slate-100 bg-slate-50/80 px-3 py-2">
              <span className="font-medium">{r.branch}</span> @ {r.commit_sha.slice(0, 7) || "—"} — {r.status}
              {r.meta?.deploy_job_ids && r.meta.deploy_job_ids.length > 0 && (
                <span className="ml-2 text-xs text-slate-500">
                  jobs: {r.meta.deploy_job_ids.join(", ")}
                </span>
              )}
            </li>
          ))}
        </ul>
        {rows.length === 0 && <p className="text-sm text-slate-500">No pushes recorded yet.</p>}
      </Panel>
    </div>
  );
}

export function UsagePageApp() {
  const teamId = useTeamId();
  const [u, setU] = useState<{
    period_days: number;
    deploy_job_count: number;
    succeeded: number;
    failed: number;
  } | null>(null);
  const [registry, setRegistry] = useState<RegistryStatusResponse | null>(null);
  const [cost, setCost] = useState<CostSummaryResponse | null>(null);
  const [extraErr, setExtraErr] = useState<string | null>(null);

  useEffect(() => {
    if (!teamId) return;
    let c = false;
    (async () => {
      try {
        const [usage, reg, co] = await Promise.all([
          apiFetch(`/api/v1/teams/${teamId}/usage?days=30`),
          apiFetch<RegistryStatusResponse>(`/api/v1/teams/${teamId}/registry/status`),
          apiFetch<CostSummaryResponse>(`/api/v1/teams/${teamId}/cost/summary`),
        ]);
        if (c) return;
        setU(usage as typeof u);
        setRegistry(reg);
        setCost(co);
        setExtraErr(null);
      } catch (e) {
        if (!c) {
          setExtraErr(e instanceof Error ? e.message : "Failed to load usage extras");
          setU(null);
          setRegistry(null);
          setCost(null);
        }
      }
    })();
    return () => {
      c = true;
    };
  }, [teamId]);

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Wallet className="h-6 w-6" strokeWidth={1.75} />}
        title="Usage"
        description="Deploy volume, registry posture, and cost placeholders—full metering arrives with agent-backed metrics."
      />
      <InlineError message={extraErr} />
      <Panel title="Deploy usage (30 days)">
        {u ? (
          <dl className="grid max-w-md grid-cols-2 gap-2 text-sm">
            <dt className="text-slate-500">Jobs</dt>
            <dd className="font-semibold">{u.deploy_job_count}</dd>
            <dt className="text-slate-500">Succeeded</dt>
            <dd>{u.succeeded}</dd>
            <dt className="text-slate-500">Failed</dt>
            <dd>{u.failed}</dd>
          </dl>
        ) : (
          <p className="text-sm text-slate-500">Loading…</p>
        )}
      </Panel>
      <Panel title="Registry integration">
        {registry ? (
          <div className="space-y-2 text-sm">
            <p className="text-slate-600">
              Built-in OCI registry is <strong>{registry.integrated ? "enabled" : "not bundled"}</strong>.{" "}
              {registry.hint}
            </p>
          </div>
        ) : (
          <p className="text-sm text-slate-500">Loading…</p>
        )}
      </Panel>
      <Panel title="Cost / showback">
        {cost ? (
          <div className="space-y-2 text-sm text-slate-600">
            <p>
              Currency: <span className="font-medium text-slate-800">{cost.currency}</span>
              {cost.synthetic_monthly_estimate != null && (
                <>
                  {" "}
                  · Estimate:{" "}
                  <span className="font-mono text-slate-800">
                    {String(cost.synthetic_monthly_estimate)}
                  </span>
                </>
              )}
            </p>
            <p>{cost.note}</p>
          </div>
        ) : (
          <p className="text-sm text-slate-500">Loading…</p>
        )}
      </Panel>
    </div>
  );
}

export function SupportPageApp() {
  const teamId = useTeamId();
  const [docs_url, setDocs] = useState("");
  const [status_url, setSt] = useState("");
  const [contact_email, setMail] = useState("");

  async function load() {
    if (!teamId) return;
    const r = await apiFetch<{
      docs_url?: string | null;
      status_url?: string | null;
      contact_email?: string | null;
    }>(`/api/v1/teams/${teamId}/support-links`);
    setDocs(r.docs_url ?? "");
    setSt(r.status_url ?? "");
    setMail(r.contact_email ?? "");
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function save(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    await apiFetch(`/api/v1/teams/${teamId}/support-links`, {
      method: "PUT",
      body: JSON.stringify({
        docs_url: docs_url || null,
        status_url: status_url || null,
        contact_email: contact_email || null,
      }),
    });
    await load();
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<LifeBuoy className="h-6 w-6" strokeWidth={1.75} />}
        title="Support"
        description="Docs, status page, and contact email surfaced to your team in the app shell."
      />
      <Panel title="Links shown to your team">
        <form onSubmit={save} className="space-y-3">
          <label className="block text-sm">
            <span className="text-slate-600">Docs URL</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={docs_url}
              onChange={(e) => setDocs(e.target.value)}
            />
          </label>
          <label className="block text-sm">
            <span className="text-slate-600">Status page URL</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={status_url}
              onChange={(e) => setSt(e.target.value)}
            />
          </label>
          <label className="block text-sm">
            <span className="text-slate-600">Contact email</span>
            <input
              type="email"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={contact_email}
              onChange={(e) => setMail(e.target.value)}
            />
          </label>
          <button type="submit" className="rounded-lg bg-brand-600 px-4 py-2 text-sm text-white">
            Save
          </button>
        </form>
      </Panel>
    </div>
  );
}
