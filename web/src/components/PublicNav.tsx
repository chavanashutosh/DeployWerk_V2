import { Link, NavLink } from "react-router-dom";
import { LayoutDashboard, LogIn, Rocket } from "lucide-react";

const linkClass = ({ isActive }: { isActive: boolean }) =>
  `rounded-lg px-3 py-2 text-sm font-medium transition ${
    isActive
      ? "bg-brand-100 text-brand-700"
      : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"
  }`;

export function PublicNav() {
  return (
    <header className="border-b border-slate-200 bg-white">
      <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-4 py-4">
        <Link to="/" className="flex items-center gap-2 font-semibold text-slate-900">
          <span className="flex h-9 w-9 items-center justify-center rounded-lg bg-brand-600 text-white">
            <Rocket className="h-5 w-5" strokeWidth={1.75} />
          </span>
          DeployWerk
        </Link>
        <nav className="hidden items-center gap-1 md:flex">
          <NavLink to="/pricing" className={linkClass}>
            Pricing
          </NavLink>
          <NavLink to="/demo" className={linkClass}>
            Sample logins
          </NavLink>
        </nav>
        <div className="flex items-center gap-2">
          <Link
            to="/login"
            className="inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-3 py-2 text-sm font-medium text-slate-700 hover:bg-slate-50"
          >
            <LogIn className="h-4 w-4" strokeWidth={1.75} />
            Sign in
          </Link>
          <Link
            to="/app"
            className="inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-brand-700"
          >
            <LayoutDashboard className="h-4 w-4" strokeWidth={1.75} />
            Dashboard
          </Link>
        </div>
      </div>
    </header>
  );
}
