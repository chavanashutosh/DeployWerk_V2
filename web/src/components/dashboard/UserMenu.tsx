import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { Building2, ChevronDown, LogOut, Shield, User } from "lucide-react";
import { useAuth } from "@/auth";

type Props = {
  teamId: string;
  orgId?: string;
};

export function UserMenu({ teamId, orgId }: Props) {
  const settingsBase = teamId ? `/app/teams/${teamId}/settings` : "";
  const { user, logout } = useAuth();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onDoc(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("click", onDoc);
    return () => document.removeEventListener("click", onDoc);
  }, []);

  const initial = user?.email?.charAt(0).toUpperCase() ?? "?";

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex items-center gap-2 rounded-md border border-slate-300 bg-white px-2 py-1.5 text-sm font-medium text-slate-800 shadow-sm hover:bg-slate-50"
        aria-expanded={open}
        aria-haspopup="menu"
      >
        <span className="flex h-8 w-8 items-center justify-center rounded-full bg-slate-200 text-sm font-semibold text-slate-800">
          {initial}
        </span>
        <span className="hidden max-w-[140px] truncate sm:inline">{user?.email}</span>
        <ChevronDown className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
      </button>
      {open && (
        <div
          className="dw-card-elevated absolute right-0 z-50 mt-1 w-56 py-1"
          role="menu"
        >
          <div className="border-b border-slate-200 px-3 py-2">
            <p className="truncate text-xs text-slate-500">Signed in</p>
            <p className="truncate text-sm font-medium text-slate-900">{user?.email}</p>
          </div>
          <Link
            to={teamId ? `${settingsBase}/general` : "/app"}
            role="menuitem"
            className="flex items-center gap-2 px-3 py-2 text-sm text-slate-700 hover:bg-slate-50"
            onClick={() => setOpen(false)}
          >
            <User className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
            Profile &amp; account
          </Link>
          {teamId ? (
            <Link
              to={`${settingsBase}/team`}
              role="menuitem"
              className="flex items-center gap-2 px-3 py-2 text-sm text-slate-700 hover:bg-slate-50"
              onClick={() => setOpen(false)}
            >
              Team settings
            </Link>
          ) : null}
          {orgId &&
          (user?.organization_admin_organization_ids?.includes(orgId) ||
            user?.is_platform_admin) ? (
            <Link
              to={`/app/orgs/${orgId}/settings`}
              role="menuitem"
              className="flex items-center gap-2 px-3 py-2 text-sm text-slate-700 hover:bg-slate-50"
              onClick={() => setOpen(false)}
            >
              <Building2 className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
              Organization
            </Link>
          ) : null}
          {user?.is_platform_admin ? (
            <Link
              to="/admin"
              role="menuitem"
              className="flex items-center gap-2 px-3 py-2 text-sm text-slate-800 hover:bg-slate-50"
              onClick={() => setOpen(false)}
            >
              <Shield className="h-4 w-4 text-slate-600" strokeWidth={1.75} />
              Super admin
            </Link>
          ) : null}
          <button
            type="button"
            role="menuitem"
            className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm text-red-600 hover:bg-red-50"
            onClick={() => {
              setOpen(false);
              logout();
            }}
          >
            <LogOut className="h-4 w-4" strokeWidth={1.75} />
            Sign out
          </button>
        </div>
      )}
    </div>
  );
}
