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
  id: string;
  name: string;
  slug: string;
  created_at: string;
  team_count: number;
};

export function AdminOrganizationsPage() {
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
          const list = await apiFetch<Row[]>(`/api/v1/admin/organizations?${params}`);
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

  const colSpan = 4;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-slate-900">Organizations</h1>
        <p className="mt-1 text-sm text-slate-600">Search by name or slug.</p>
      </div>
      <AdminSearchField
        id="admin-orgs-search"
        label="Search organizations"
        placeholder="Name or slug…"
        value={q}
        onChange={setQ}
        icon={Search}
      />
      {err && <p className="text-sm text-red-600">{err}</p>}
      <AdminTableWrap>
        <table className="w-full min-w-[560px] text-sm">
          <AdminThead>
            <tr>
              <th className="px-4 py-3 font-medium">Name</th>
              <th className="px-4 py-3 font-medium">Slug</th>
              <th className="px-4 py-3 font-medium">Teams</th>
              <th className="px-4 py-3 font-medium">Created</th>
            </tr>
          </AdminThead>
          <tbody>
            {loading ? (
              <AdminLoadingRow colSpan={colSpan} />
            ) : rows.length === 0 ? (
              <AdminEmptyRow colSpan={colSpan} message="No organizations match this search." />
            ) : (
              rows.map((r) => (
                <tr
                  key={r.id}
                  className="border-b border-slate-100 even:bg-slate-50/40 hover:bg-violet-50/30"
                >
                  <td className="px-4 py-2">
                    <AdminViewLink to={`/admin/organizations/${r.id}`} label={r.name} icon={Eye} />
                  </td>
                  <td className="px-4 py-2 text-slate-600">{r.slug}</td>
                  <td className="px-4 py-2">{r.team_count}</td>
                  <td className="px-4 py-2 text-slate-600">{new Date(r.created_at).toLocaleString()}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </AdminTableWrap>
    </div>
  );
}
