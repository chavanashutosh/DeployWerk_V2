import { NavLink, Outlet } from "react-router-dom";
import {
  Activity,
  BarChart3,
  Building2,
  ClipboardList,
  CreditCard,
  Server,
  Shield,
  Tag,
  Users,
} from "lucide-react";
import { useAuth } from "@/auth";

const linkCls = ({ isActive }: { isActive: boolean }) =>
  `flex items-center gap-2 rounded-md border-l-2 py-2 pl-2 pr-3 text-sm font-medium transition-colors ${
    isActive
      ? "border-white bg-slate-800 text-white"
      : "border-transparent text-slate-400 hover:bg-slate-800/60 hover:text-slate-200"
  }`;

export function AdminShell() {
  const { user, logout } = useAuth();

  return (
    <div className="min-h-screen bg-slate-50">
      <div className="flex min-h-screen">
        <aside className="w-56 shrink-0 border-r border-slate-800 bg-slate-950 px-3 py-6 md:w-60">
          <div className="mb-5 flex items-center gap-2 rounded-lg border border-slate-800 bg-slate-900/40 px-3 py-3">
            <Shield className="h-5 w-5 shrink-0 text-slate-300" strokeWidth={1.75} />
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-widest text-slate-500">Super admin</p>
              <p className="text-sm font-semibold text-white">DeployWerk</p>
              <p className="mt-1 text-[10px] leading-snug text-slate-500">
                Instance operators only — not organization or team administrators.
              </p>
            </div>
          </div>
          <nav className="flex flex-col gap-0.5 rounded-lg border border-slate-800/80 bg-slate-900/20 p-1.5">
            <NavLink to="/admin" end className={linkCls}>
              <BarChart3 className="h-4 w-4" strokeWidth={1.75} />
              Analytics & overview
            </NavLink>
            <NavLink to="/admin/users" className={linkCls}>
              <Users className="h-4 w-4" strokeWidth={1.75} />
              Users
            </NavLink>
            <NavLink to="/admin/organizations" className={linkCls}>
              <Building2 className="h-4 w-4" strokeWidth={1.75} />
              Organizations
            </NavLink>
            <NavLink to="/admin/teams" className={linkCls}>
              <Server className="h-4 w-4" strokeWidth={1.75} />
              Teams
            </NavLink>
            <NavLink to="/admin/billing" className={linkCls}>
              <CreditCard className="h-4 w-4" strokeWidth={1.75} />
              Billing
            </NavLink>
            <NavLink to="/admin/pricing" className={linkCls}>
              <Tag className="h-4 w-4" strokeWidth={1.75} />
              Pricing
            </NavLink>
            <NavLink to="/admin/audit" className={linkCls}>
              <ClipboardList className="h-4 w-4" strokeWidth={1.75} />
              Audit log
            </NavLink>
            <NavLink to="/admin/system" className={linkCls}>
              <Activity className="h-4 w-4" strokeWidth={1.75} />
              System
            </NavLink>
          </nav>
          <div className="mt-6 border-t border-slate-800 px-3 pt-4">
            <p className="truncate text-xs text-slate-500">{user?.email}</p>
            <NavLink to="/app" className="mt-2 block text-sm text-slate-400 hover:text-white">
              ← Back to app
            </NavLink>
            <button
              type="button"
              className="mt-2 text-sm text-slate-500 hover:text-slate-200"
              onClick={() => logout()}
            >
              Sign out
            </button>
          </div>
        </aside>
        <main className="flex-1 overflow-auto p-6 md:p-8">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
