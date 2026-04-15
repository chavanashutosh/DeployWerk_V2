import type { LucideIcon } from "lucide-react";
import {
  Building2,
  CreditCard,
  LayoutList,
  Rocket,
  ScrollText,
  UserPlus,
  Users,
  UsersRound,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { apiFetch } from "@/api";
import { formatAdminListError } from "./adminUi";

type Overview = {
  days: number;
  total_users: number;
  total_organizations: number;
  total_teams: number;
  user_signups_by_day: [string, number][];
  teams_created_by_day: [string, number][];
  deploy_jobs_by_day: [string, string, number][];
  rum_events_by_day: [string, number][];
  billing_by_plan: [string, string, number][];
};

function aggregateDeployJobsByDay(rows: [string, string, number][]): [string, number][] {
  const m = new Map<string, number>();
  for (const [d, , n] of rows) {
    m.set(d, (m.get(d) ?? 0) + n);
  }
  return [...m.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

function deployTotalsByStatus(rows: [string, string, number][]): [string, number][] {
  const m = new Map<string, number>();
  for (const [, st, n] of rows) {
    m.set(st, (m.get(st) ?? 0) + n);
  }
  return [...m.entries()].sort((a, b) => b[1] - a[1]);
}

export function AdminDashboardPage() {
  const [data, setData] = useState<Overview | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [days, setDays] = useState(30);

  useEffect(() => {
    let c = false;
    (async () => {
      try {
        const o = await apiFetch<Overview>(`/api/v1/admin/analytics/overview?days=${days}`);
        if (!c) {
          setData(o);
          setErr(null);
        }
      } catch (e) {
        if (!c) setErr(formatAdminListError(e));
      }
    })();
    return () => {
      c = true;
    };
  }, [days]);

  const deployByDay = useMemo(
    () => (data ? aggregateDeployJobsByDay(data.deploy_jobs_by_day) : []),
    [data],
  );
  const deployByStatus = useMemo(
    () => (data ? deployTotalsByStatus(data.deploy_jobs_by_day) : []),
    [data],
  );

  return (
    <div className="space-y-8">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold text-slate-900">Platform overview</h1>
          <p className="mt-1 text-sm text-slate-600">
            Operator analytics (distinct from per-team Analytics in the app sidebar).
          </p>
        </div>
        <label className="flex items-center gap-2 text-sm text-slate-600">
          <span className="text-xs font-medium uppercase tracking-wide text-slate-500">Period (days)</span>
          <select
            className="rounded-lg border border-slate-200 bg-white px-2 py-1.5 text-slate-900 focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
            value={days}
            onChange={(e) => setDays(Number(e.target.value))}
          >
            {[7, 14, 30, 90].map((d) => (
              <option key={d} value={d}>
                {d}
              </option>
            ))}
          </select>
        </label>
      </div>

      {err && <p className="text-sm text-red-600">{err}</p>}

      {data && (
        <>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <StatCard label="Users" value={data.total_users} icon={Users} />
            <StatCard label="Organizations" value={data.total_organizations} icon={Building2} />
            <StatCard label="Teams" value={data.total_teams} icon={UsersRound} />
            <div className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
              <p className="text-xs font-medium uppercase tracking-wide text-slate-500">Shortcuts</p>
              <ul className="mt-2 space-y-2 text-sm">
                <li>
                  <Link
                    className="inline-flex items-center gap-2 text-violet-700 hover:text-violet-900 hover:underline"
                    to="/admin/users"
                  >
                    <Users className="h-4 w-4 shrink-0 text-violet-600" strokeWidth={1.75} aria-hidden />
                    Directory: users
                  </Link>
                </li>
                <li>
                  <Link
                    className="inline-flex items-center gap-2 text-violet-700 hover:text-violet-900 hover:underline"
                    to="/admin/billing"
                  >
                    <CreditCard className="h-4 w-4 shrink-0 text-violet-600" strokeWidth={1.75} aria-hidden />
                    Billing grid
                  </Link>
                </li>
                <li>
                  <Link
                    className="inline-flex items-center gap-2 text-violet-700 hover:text-violet-900 hover:underline"
                    to="/admin/audit"
                  >
                    <ScrollText className="h-4 w-4 shrink-0 text-violet-600" strokeWidth={1.75} aria-hidden />
                    Audit log
                  </Link>
                </li>
              </ul>
            </div>
          </div>

          {deployByStatus.length > 0 && (
            <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
              <div className="flex items-center gap-2">
                <Rocket className="h-5 w-5 text-violet-600" strokeWidth={1.75} aria-hidden />
                <h2 className="text-lg font-semibold text-slate-900">Deploy jobs (period total by status)</h2>
              </div>
              <p className="mt-1 text-sm text-slate-600">Summed over the selected window from deploy activity by day.</p>
              <ul className="mt-4 flex flex-wrap gap-3">
                {deployByStatus.map(([status, n]) => (
                  <li
                    key={status}
                    className="rounded-lg border border-slate-100 bg-slate-50 px-3 py-2 text-sm"
                  >
                    <span className="font-medium text-slate-900">{status}</span>
                    <span className="ml-2 tabular-nums text-slate-600">{n.toLocaleString()}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
            <div className="flex items-center gap-2">
              <CreditCard className="h-5 w-5 text-violet-600" strokeWidth={1.75} aria-hidden />
              <h2 className="text-lg font-semibold text-slate-900">Billing snapshot</h2>
            </div>
            <p className="mt-1 text-sm text-slate-600">Teams with a billing row, grouped by plan and status.</p>
            <div className="mt-4 overflow-auto">
              <table className="w-full min-w-[360px] text-sm">
                <thead>
                  <tr className="border-b border-slate-100 text-left text-slate-500">
                    <th className="pb-2 pr-4 font-medium">Plan</th>
                    <th className="pb-2 pr-4 font-medium">Status</th>
                    <th className="pb-2 font-medium">Teams</th>
                  </tr>
                </thead>
                <tbody>
                  {data.billing_by_plan.map(([plan, status, n]) => (
                    <tr key={`${plan}-${status}`} className="border-b border-slate-50">
                      <td className="py-2 pr-4">{plan}</td>
                      <td className="py-2 pr-4">{status}</td>
                      <td className="py-2">{n}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          <div className="grid gap-6 lg:grid-cols-2">
            <DayBarBlock title="User signups by day" rows={data.user_signups_by_day} icon={UserPlus} />
            <DayBarBlock title="Teams created by day" rows={data.teams_created_by_day} icon={UsersRound} />
            <DayBarBlock title="Deploy jobs by day (all statuses)" rows={deployByDay} icon={Rocket} />
            <DayBarBlock title="RUM events by day" rows={data.rum_events_by_day} icon={LayoutList} />
          </div>

          <DeployJobsTable rows={data.deploy_jobs_by_day} />
        </>
      )}
    </div>
  );
}

function StatCard({ label, value, icon: Icon }: { label: string; value: number; icon: LucideIcon }) {
  return (
    <div className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
      <div className="flex items-start justify-between gap-2">
        <p className="text-xs font-medium uppercase tracking-wide text-slate-500">{label}</p>
        <Icon className="h-5 w-5 shrink-0 text-violet-500/90" strokeWidth={1.75} aria-hidden />
      </div>
      <p className="mt-1 text-2xl font-semibold tabular-nums text-slate-900">{value.toLocaleString()}</p>
    </div>
  );
}

function DayBarBlock({
  title,
  rows,
  icon: Icon,
}: {
  title: string;
  rows: [string, number][];
  icon: LucideIcon;
}) {
  const max = Math.max(1, ...rows.map(([, n]) => n));
  return (
    <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
      <div className="flex items-center gap-2">
        <Icon className="h-5 w-5 text-violet-600" strokeWidth={1.75} aria-hidden />
        <h2 className="text-lg font-semibold text-slate-900">{title}</h2>
      </div>
      {rows.length === 0 ? (
        <p className="mt-4 text-sm text-slate-500">No data in this range.</p>
      ) : (
        <>
          <div className="mt-4 overflow-x-auto rounded-lg border border-slate-100 bg-slate-50 p-3">
            <div
              className="flex h-36 items-end gap-px"
              style={{ minWidth: Math.max(rows.length * 4, 120) }}
              role="img"
              aria-label={title}
            >
              {rows.map(([d, n]) => (
                <div key={d} className="min-w-[3px] flex-1" title={`${d}: ${n.toLocaleString()}`}>
                  <div
                    className="w-full min-h-0 rounded-sm bg-violet-500/85 transition-[height] hover:bg-violet-600"
                    style={{
                      height: `${(n / max) * 100}%`,
                      minHeight: n > 0 ? 3 : 0,
                    }}
                  />
                </div>
              ))}
            </div>
          </div>
          <p className="mt-2 text-xs text-slate-500">Hover a bar for date and count. Range: {rows.length} days.</p>
        </>
      )}
    </section>
  );
}

function DeployJobsTable({ rows }: { rows: [string, string, number][] }) {
  return (
    <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
      <div className="flex items-center gap-2">
        <Rocket className="h-5 w-5 text-slate-500" strokeWidth={1.75} aria-hidden />
        <h2 className="text-lg font-semibold text-slate-900">Deploy jobs by day (detail)</h2>
      </div>
      <div className="mt-4 max-h-64 overflow-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-100 text-left text-slate-500">
              <th className="pb-2 font-medium">Date</th>
              <th className="pb-2 font-medium">Status</th>
              <th className="pb-2 font-medium">Count</th>
            </tr>
          </thead>
          <tbody>
            {rows.map(([d, st, n]) => (
              <tr key={`${d}-${st}`} className="border-b border-slate-50">
                <td className="py-1.5">{d}</td>
                <td className="py-1.5">{st}</td>
                <td className="py-1.5 tabular-nums">{n}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
