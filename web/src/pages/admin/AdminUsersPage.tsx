import { Eye, Search, ShieldCheck } from "lucide-react";
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
  email: string;
  name: string | null;
  created_at: string;
  is_platform_admin: boolean;
};

export function AdminUsersPage() {
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
          const list = await apiFetch<Row[]>(`/api/v1/admin/users?${params}`);
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
        <h1 className="text-2xl font-semibold text-slate-900">Users</h1>
        <p className="mt-1 text-sm text-slate-600">Search by email or name.</p>
      </div>
      <AdminSearchField
        id="admin-users-search"
        label="Search directory"
        placeholder="Email or name…"
        value={q}
        onChange={setQ}
        icon={Search}
      />
      {err && <p className="text-sm text-red-600">{err}</p>}
      <AdminTableWrap>
        <table className="w-full min-w-[720px] text-sm">
          <AdminThead>
            <tr>
              <th className="px-4 py-3 font-medium">Email</th>
              <th className="px-4 py-3 font-medium">Name</th>
              <th className="px-4 py-3 font-medium">Created</th>
              <th className="px-4 py-3 font-medium">Super admin</th>
            </tr>
          </AdminThead>
          <tbody>
            {loading ? (
              <AdminLoadingRow colSpan={colSpan} />
            ) : rows.length === 0 ? (
              <AdminEmptyRow colSpan={colSpan} message="No users match this search." />
            ) : (
              rows.map((r) => (
                <tr
                  key={r.id}
                  className="border-b border-slate-100 even:bg-slate-50/40 hover:bg-violet-50/30"
                >
                  <td className="px-4 py-2">
                    <AdminViewLink to={`/admin/users/${r.id}`} label={r.email} icon={Eye} />
                  </td>
                  <td className="px-4 py-2 text-slate-600">{r.name ?? "—"}</td>
                  <td className="px-4 py-2 text-slate-600">
                    {new Date(r.created_at).toLocaleString()}
                  </td>
                  <td className="px-4 py-2">
                    {r.is_platform_admin ? (
                      <span className="inline-flex items-center gap-1 text-emerald-700">
                        <ShieldCheck className="h-4 w-4" strokeWidth={1.75} aria-hidden />
                        Yes
                      </span>
                    ) : (
                      "—"
                    )}
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
