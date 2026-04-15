import { Eye, Search } from "lucide-react";
import { useEffect, useState } from "react";
import { apiFetch } from "@/api";
import {
  AdminEmptyRow,
  AdminLoadingRow,
  AdminSearchField,
  AdminTableWrap,
  AdminThead,
  AdminViewLink,
  formatAdminListError,
} from "./adminUi";

type Row = {
  team_id: string;
  team_name: string;
  team_slug: string;
  organization_id: string;
  org_name: string;
  plan_name: string;
  status: string;
  payment_provider: string;
  provider_customer_id: string | null;
  stripe_customer_id?: string | null;
  updated_at: string;
};

export function AdminBillingPage() {
  const [q, setQ] = useState("");
  const [rows, setRows] = useState<Row[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const t = setTimeout(() => {
      void (async () => {
        setLoading(true);
        try {
          const params = new URLSearchParams({ limit: "200", offset: "0" });
          if (q.trim()) params.set("q", q.trim());
          const list = await apiFetch<Row[]>(`/api/v1/admin/billing?${params}`);
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

  const colSpan = 7;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-slate-900">Billing directory</h1>
        <p className="mt-1 text-sm text-slate-600">All teams with plan, status, and payment provider metadata.</p>
      </div>
      <AdminSearchField
        id="admin-billing-search"
        label="Search billing rows"
        placeholder="Team, org, plan…"
        value={q}
        onChange={setQ}
        icon={Search}
      />
      {err && <p className="text-sm text-red-600">{err}</p>}
      <AdminTableWrap>
        <table className="w-full min-w-[960px] text-sm">
          <AdminThead>
            <tr>
              <th className="px-4 py-3 font-medium">Team</th>
              <th className="px-4 py-3 font-medium">Organization</th>
              <th className="px-4 py-3 font-medium">Plan</th>
              <th className="px-4 py-3 font-medium">Status</th>
              <th className="px-4 py-3 font-medium">Provider</th>
              <th className="px-4 py-3 font-medium">Customer ref</th>
              <th className="px-4 py-3 font-medium">Updated</th>
            </tr>
          </AdminThead>
          <tbody>
            {loading ? (
              <AdminLoadingRow colSpan={colSpan} />
            ) : rows.length === 0 ? (
              <AdminEmptyRow colSpan={colSpan} message="No billing rows match this search." />
            ) : (
              rows.map((r) => (
                <tr
                  key={r.team_id}
                  className="border-b border-slate-100 even:bg-slate-50/40 hover:bg-violet-50/30"
                >
                  <td className="px-4 py-2">
                    <AdminViewLink to={`/admin/teams/${r.team_id}`} label={r.team_name} icon={Eye} />
                  </td>
                  <td className="px-4 py-2 text-slate-600">{r.org_name}</td>
                  <td className="px-4 py-2">{r.plan_name}</td>
                  <td className="px-4 py-2">{r.status}</td>
                  <td className="px-4 py-2">{r.payment_provider}</td>
                  <td className="max-w-[200px] truncate px-4 py-2 font-mono text-xs text-slate-600">
                    {r.provider_customer_id ?? r.stripe_customer_id ?? "—"}
                  </td>
                  <td className="px-4 py-2 text-slate-600">{new Date(r.updated_at).toLocaleString()}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </AdminTableWrap>
    </div>
  );
}
