import { ArrowLeft, Eye } from "lucide-react";
import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { apiFetch } from "@/api";
import { AdminViewLink, formatAdminListError } from "./adminUi";

type Detail = {
  organization: {
    id: string;
    name: string;
    slug: string;
    created_at: string;
    team_count: number;
  };
  members: { user_id: string; email: string; name: string | null; role: string }[];
  teams: { id: string; name: string; slug: string }[];
};

export function AdminOrganizationDetailPage() {
  const { id = "" } = useParams();
  const [d, setD] = useState<Detail | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    if (!id) return;
    let c = false;
    (async () => {
      try {
        const x = await apiFetch<Detail>(`/api/v1/admin/organizations/${id}`);
        if (!c) {
          setD(x);
          setErr(null);
        }
      } catch (e) {
        if (!c) setErr(formatAdminListError(e));
      }
    })();
    return () => {
      c = true;
    };
  }, [id]);

  if (!d && !err) return <p className="text-slate-600">Loading…</p>;
  if (err && !d)
    return (
      <p className="text-red-600">
        {err}{" "}
        <Link to="/admin/organizations" className="inline-flex items-center gap-1 text-violet-700 hover:underline">
          <ArrowLeft className="h-4 w-4" strokeWidth={1.75} aria-hidden />
          Organizations
        </Link>
      </p>
    );
  if (!d) return null;

  return (
    <div className="space-y-8">
      <div>
        <Link
          to="/admin/organizations"
          className="inline-flex items-center gap-1 text-sm text-violet-700 hover:underline"
        >
          <ArrowLeft className="h-4 w-4" strokeWidth={1.75} aria-hidden />
          Organizations
        </Link>
        <h1 className="mt-2 text-2xl font-semibold text-slate-900">{d.organization.name}</h1>
        <p className="mt-1 text-sm text-slate-600">
          {d.organization.slug} · {d.organization.team_count} teams
        </p>
      </div>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Teams</h2>
        <ul className="mt-3 space-y-2 text-sm">
          {d.teams.map((t) => (
            <li key={t.id}>
              <AdminViewLink to={`/admin/teams/${t.id}`} label={t.name} icon={Eye} />{" "}
              <span className="text-slate-500">{t.slug}</span>
            </li>
          ))}
        </ul>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Members</h2>
        <ul className="mt-3 space-y-2 text-sm">
          {d.members.map((m) => (
            <li key={m.user_id}>
              <AdminViewLink to={`/admin/users/${m.user_id}`} label={m.email} icon={Eye} />{" "}
              <span className="text-slate-500">
                ({m.role}) {m.name ? `· ${m.name}` : ""}
              </span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
