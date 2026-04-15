import { FormEvent, useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { Mail } from "lucide-react";
import { apiFetch } from "@/api";
import { InlineError, LoadingBlock, PageHeader } from "@/components/ui";

type MailDomainRow = {
  id: string;
  domain: string;
  status: string;
  created_at: string;
};

export function MailDomainsSettingsPage() {
  const { teamId = "" } = useParams();
  const [rows, setRows] = useState<MailDomainRow[] | null>(null);
  const [domain, setDomain] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [dnsPreviewId, setDnsPreviewId] = useState<string | null>(null);
  const [dnsJson, setDnsJson] = useState<string | null>(null);

  async function load() {
    if (!teamId) return;
    const list = await apiFetch<MailDomainRow[]>(`/api/v1/teams/${teamId}/mail/domains`);
    setRows(list);
  }

  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!teamId) return;
      try {
        await load();
        if (!cancelled) setErr(null);
      } catch (e) {
        if (!cancelled) setErr(e instanceof Error ? e.message : "Failed to load");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId]);

  async function onAdd(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/mail/domains`, {
        method: "POST",
        body: JSON.stringify({ domain: domain.trim() }),
      });
      setDomain("");
      await load();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Create failed");
    } finally {
      setPending(false);
    }
  }

  async function onDelete(id: string) {
    if (!teamId) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/mail/domains/${id}`, { method: "DELETE" });
      if (dnsPreviewId === id) {
        setDnsPreviewId(null);
        setDnsJson(null);
      }
      await load();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Delete failed");
    } finally {
      setPending(false);
    }
  }

  async function showDnsStub(id: string) {
    if (!teamId) return;
    setDnsPreviewId(id);
    setDnsJson(null);
    try {
      const j = await apiFetch<Record<string, unknown>>(
        `/api/v1/teams/${teamId}/mail/domains/${id}/dns-check`,
      );
      setDnsJson(JSON.stringify(j, null, 2));
    } catch (e2) {
      setDnsJson(e2 instanceof Error ? e2.message : "Failed");
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Mail className="h-6 w-6" strokeWidth={1.75} />}
        title="Mail domains"
        description="Register domains for the team mail platform (Phase 1). DNS validation is a stub; follow spec/08 for required records."
      />
      <InlineError message={err} />
      <form onSubmit={onAdd} className="flex flex-wrap items-end gap-2">
        <label className="text-sm">
          <span className="text-slate-600">Domain</span>
          <input
            className="mt-1 block rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
            value={domain}
            onChange={(e) => setDomain(e.target.value)}
            placeholder="mail.example.com"
            required
          />
        </label>
        <button
          type="submit"
          disabled={pending}
          className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white disabled:opacity-60"
        >
          Add
        </button>
      </form>
      {rows === null && !err && <LoadingBlock label="Loading domains…" />}
      {rows && rows.length === 0 && <p className="text-sm text-slate-600">No domains yet.</p>}
      {rows && rows.length > 0 && (
        <ul className="divide-y divide-slate-100 rounded-xl border border-slate-200 bg-white">
          {rows.map((r) => (
            <li key={r.id} className="flex flex-wrap items-center justify-between gap-2 px-4 py-3 text-sm">
              <div>
                <span className="font-mono font-medium text-slate-900">{r.domain}</span>
                <span className="ml-2 rounded bg-slate-100 px-2 py-0.5 text-xs text-slate-600">{r.status}</span>
              </div>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  className="rounded border border-slate-200 px-2 py-1 text-xs hover:bg-slate-50"
                  onClick={() => void showDnsStub(r.id)}
                >
                  DNS stub
                </button>
                <button
                  type="button"
                  className="rounded border border-red-200 px-2 py-1 text-xs text-red-800 hover:bg-red-50"
                  onClick={() => void onDelete(r.id)}
                  disabled={pending}
                >
                  Remove
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
      {dnsPreviewId && dnsJson && (
        <div className="dw-card p-4">
          <p className="mb-2 text-xs font-medium text-slate-500">DNS check (placeholder)</p>
          <pre className="max-h-64 overflow-auto rounded-lg bg-slate-900 p-3 font-mono text-xs text-slate-100">
            {dnsJson}
          </pre>
        </div>
      )}
    </div>
  );
}
