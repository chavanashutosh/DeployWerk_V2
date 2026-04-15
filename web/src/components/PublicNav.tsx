import { Link, NavLink } from "react-router-dom";
import { LayoutDashboard, LogIn, Rocket } from "lucide-react";

const linkClass = ({ isActive }: { isActive: boolean }) =>
  `rounded-md px-3 py-2 text-sm font-medium transition ${
    isActive
      ? "bg-slate-100 text-slate-900"
      : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"
  }`;

export function PublicNav() {
  return (
    <header className="border-b border-slate-200 bg-white shadow-sm">
      <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-4 py-3">
        <Link to="/" className="flex items-center gap-2.5 font-semibold text-slate-900">
          <span className="flex h-9 w-9 items-center justify-center rounded-md border border-slate-200 bg-slate-900 text-white shadow-sm">
            <Rocket className="h-4 w-4" strokeWidth={1.75} />
          </span>
          DeployWerk
        </Link>
        <nav className="hidden items-center gap-0.5 md:flex">
          <NavLink to="/pricing" className={linkClass}>
            Pricing
          </NavLink>
          <NavLink to="/demo" className={linkClass}>
            Sample logins
          </NavLink>
        </nav>
        <div className="flex items-center gap-2">
          <Link to="/login" className="dw-btn-secondary gap-1.5 px-3 py-2 text-sm">
            <LogIn className="h-4 w-4" strokeWidth={1.75} />
            Sign in
          </Link>
          <Link to="/app" className="dw-btn-primary gap-1.5 px-3 py-2 text-sm">
            <LayoutDashboard className="h-4 w-4" strokeWidth={1.75} />
            Dashboard
          </Link>
        </div>
      </div>
    </header>
  );
}
