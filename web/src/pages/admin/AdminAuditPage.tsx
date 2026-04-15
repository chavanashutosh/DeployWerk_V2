import { Filter } from "lucide-react";
import { useEffect, useState } from "react";
import { apiFetch } from "@/api";
import {
  AdminEmptyRow,
  AdminLoadingRow,
  AdminSearchField,
  AdminTableWrap,
  AdminThead,
  formatAdminListError,
} from "./adminUi";

type Row = {
  id: string;
  actor_user_id: string;
  action: string;
  entity_type: string;
  entity_id: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
};

export function AdminAuditPage() {
  const [q, setQ] = useState("");
  const [rows, setRows] = useState<Row[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const t = setTimeout(() => {
      void (async () => {
        setLoading(true);
        try {
          const params = new URLSearchParams({ limit: "100", offset: "0" });
          if (q.trim()) params.set("q", q.trim());
          const list = await apiFetch<Row[]>(`/api/v1/admin/audit-log?${params}`);
          setRows(list);
          setErr(null);
        } catch (e) {
          setErr(formatAdminListError(e));
        } finally {
          setLoading(false);
        }
      })();
    }, 250);
    return () => clearTimeout(t);
  }, [q]);

  const colSpan = 5;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-slate-900">Audit log</h1>
        <p className="mt-1 text-sm text-slate-600">Super admin mutations and billing updates.</p>
      </div>
      <AdminSearchField
        id="admin-audit-filter"
        label="Filter events"
        placeholder="Action or entity type…"
        value={q}
        onChange={setQ}
        icon={Filter}
      />
      {err && <p className="text-sm text-red-600">{err}</p>}
      <AdminTableWrap>
        <table className="w-full min-w-[800px] text-sm">
          <AdminThead>
            <tr>
              <th className="px-4 py-3 font-medium">Time</th>
              <th className="px-4 py-3 font-medium">Actor</th>
              <th className="px-4 py-3 font-medium">Action</th>
              <th className="px-4 py-3 font-medium">Entity</th>
              <th className="px-4 py-3 font-medium">Metadata</th>
            </tr>
          </AdminThead>
          <tbody>
            {loading ? (
              <AdminLoadingRow colSpan={colSpan} />
            ) : rows.length === 0 ? (
              <AdminEmptyRow colSpan={colSpan} message="No audit events match this filter." />
            ) : (
              rows.map((r) => (
                <tr
                  key={r.id}
                  className="border-b border-slate-100 align-top even:bg-slate-50/40 hover:bg-violet-50/30"
                >
                  <td className="px-4 py-2 text-slate-600">{new Date(r.created_at).toLocaleString()}</td>
                  <td className="px-4 py-2 font-mono text-xs">{r.actor_user_id}</td>
                  <td className="px-4 py-2">{r.action}</td>
                  <td className="px-4 py-2">
                    {r.entity_type}
                    {r.entity_id ? (
                      <span className="mt-1 block font-mono text-xs text-slate-500">{r.entity_id}</span>
                    ) : null}
                  </td>
                  <td className="max-w-md px-4 py-2">
                    <pre className="overflow-auto text-xs text-slate-600">
                      {JSON.stringify(r.metadata, null, 0)}
                    </pre>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </AdminTableWrap>
    </div>
  );
}
