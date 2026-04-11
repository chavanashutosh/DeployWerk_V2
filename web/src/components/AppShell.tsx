import { Link, NavLink, Outlet } from "react-router-dom";
import { LayoutDashboard, LogOut, Server } from "lucide-react";
import { useAuth } from "@/auth";

const navCls = ({ isActive }: { isActive: boolean }) =>
  `flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium ${
    isActive ? "bg-brand-100 text-brand-800" : "text-slate-600 hover:bg-slate-100"
  }`;

export function AppShell() {
  const { user, logout } = useAuth();

  return (
    <div className="min-h-screen bg-slate-50">
      <div className="mx-auto flex max-w-6xl gap-8 px-4 py-8">
        <aside className="hidden w-56 shrink-0 md:block">
          <div className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm">
            <p className="truncate text-xs font-medium uppercase tracking-wide text-slate-500">
              Signed in
            </p>
            <p className="truncate text-sm font-semibold text-slate-900">{user?.email}</p>
            <nav className="mt-4 flex flex-col gap-1">
              <NavLink to="/app" end className={navCls}>
                <LayoutDashboard className="h-4 w-4" strokeWidth={1.75} />
                Overview
              </NavLink>
              <span
                className="flex cursor-not-allowed items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-400"
                title="Coming in later phases"
              >
                <Server className="h-4 w-4" strokeWidth={1.75} />
                Servers
              </span>
            </nav>
            <div className="mt-6 border-t border-slate-100 pt-4">
              <Link
                to="/"
                className="block text-sm text-slate-600 hover:text-slate-900"
              >
                Public site
              </Link>
              <button
                type="button"
                onClick={logout}
                className="mt-2 flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm font-medium text-red-600 hover:bg-red-50"
              >
                <LogOut className="h-4 w-4" strokeWidth={1.75} />
                Sign out
              </button>
            </div>
          </div>
        </aside>
        <div className="min-w-0 flex-1">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
