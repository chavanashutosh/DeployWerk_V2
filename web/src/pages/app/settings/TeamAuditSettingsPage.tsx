import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { ClipboardList } from "lucide-react";
import { apiFetch } from "@/api";
import { InlineError, LoadingBlock, PageHeader } from "@/components/ui";

type AuditRow = {
  id: string;
  actor_user_id: string;
  action: string;
  entity_type: string;
  entity_id: string | null;
  metadata: Record<string, unknown>;
  source_ip: string | null;
  created_at: string;
};

export function TeamAuditSettingsPage() {
  const { teamId = "" } = useParams();
  const [rows, setRows] = useState<AuditRow[] | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (!teamId) return;
      try {
        const list = await apiFetch<AuditRow[]>(`/api/v1/teams/${teamId}/audit-log?limit=100`);
        if (!cancelled) {
          setRows(list);
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) setErr(e instanceof Error ? e.message : "Failed to load audit log");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId]);

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<ClipboardList className="h-6 w-6" strokeWidth={1.75} />}
        title="Team audit log"
        description="Recent security-relevant actions for this team (owners/admins only)."
      />
      <InlineError message={err} />
      {rows === null && !err && <LoadingBlock label="Loading…" />}
      {rows && rows.length === 0 && (
        <p className="text-sm text-slate-600">No audit entries yet.</p>
      )}
      {rows && rows.length > 0 && (
        <div className="dw-card overflow-x-auto p-4">
          <table className="min-w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-200 text-xs font-medium uppercase tracking-wide text-slate-500">
                <th className="py-2 pr-4">When</th>
                <th className="py-2 pr-4">Action</th>
                <th className="py-2 pr-4">Entity</th>
                <th className="py-2 pr-4">Actor</th>
                <th className="py-2 pr-4">IP</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-100">
              {rows.map((r) => (
                <tr key={r.id} className="align-top">
                  <td className="py-2 pr-4 whitespace-nowrap text-slate-700">
                    {new Date(r.created_at).toLocaleString()}
                  </td>
                  <td className="py-2 pr-4 font-mono text-xs text-slate-800">{r.action}</td>
                  <td className="py-2 pr-4 text-xs text-slate-600">
                    {r.entity_type}
                    {r.entity_id ? (
                      <span className="ml-1 font-mono text-slate-800">{r.entity_id.slice(0, 8)}…</span>
                    ) : null}
                  </td>
                  <td className="py-2 pr-4 font-mono text-xs text-slate-600">{r.actor_user_id.slice(0, 8)}…</td>
                  <td className="py-2 pr-4 font-mono text-xs text-slate-600">{r.source_ip ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
