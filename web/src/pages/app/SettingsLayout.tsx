import { NavLink, Outlet, useOutletContext, useParams } from "react-router-dom";
import type { AppShellOutletContext } from "@/appShellContext";

const subCls = ({ isActive }: { isActive: boolean }) =>
  `block rounded-lg px-3 py-2 text-sm font-medium ${
    isActive ? "bg-brand-100 text-brand-800" : "text-slate-600 hover:bg-slate-100"
  }`;

export function SettingsLayout() {
  const { teamId = "" } = useParams();
  const base = `/app/teams/${teamId}/settings`;
  const { teams } = useOutletContext<AppShellOutletContext>();
  const orgId = teams.find((t) => t.id === teamId)?.organization_id;

  return (
    <div className="flex flex-col gap-8 lg:flex-row lg:items-start">
      <aside className="w-full shrink-0 lg:sticky lg:top-4 lg:w-56">
        <div className="dw-card rounded-xl p-4">
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500">Settings</h2>
          <nav className="flex flex-col gap-0.5">
          <NavLink to={`${base}/general`} className={subCls}>
            General
          </NavLink>
          <NavLink to={`${base}/team`} className={subCls}>
            Team
          </NavLink>
          <NavLink to={`${base}/secrets`} className={subCls}>
            Secrets
          </NavLink>
          <NavLink to={`${base}/audit-log`} className={subCls}>
            Audit log
          </NavLink>
          <NavLink to={`${base}/mail`} className={subCls}>
            Email &amp; mail
          </NavLink>
          <NavLink to={`${base}/mail-domains`} className={subCls}>
            Mail domains
          </NavLink>
          {orgId ? (
            <NavLink to={`/app/orgs/${orgId}/settings`} className={subCls}>
              Organization
            </NavLink>
          ) : null}
          <NavLink to="/app/settings/tokens" className={subCls}>
            API tokens
          </NavLink>
          <NavLink to={`${base}/notifications`} className={subCls}>
            Notifications
          </NavLink>
          <NavLink to={`${base}/domains`} className={subCls}>
            Domains &amp; DNS
          </NavLink>
        </nav>
        </div>
      </aside>
      <div className="min-w-0 flex-1">
        <Outlet />
      </div>
    </div>
  );
}
