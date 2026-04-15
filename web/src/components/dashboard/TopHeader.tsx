import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { Search } from "lucide-react";
import type { Organization, Team } from "@/api";
import { UserMenu } from "./UserMenu";

type Props = {
  teamId: string;
  title?: string;
  onMenuClick?: () => void;
  teams: Team[];
  organizations: Organization[];
  effectiveOrgId: string;
  effectiveTeamId: string;
  onOrgChange: (id: string) => void;
  onTeamChange: (id: string) => void;
};

export function TopHeader({
  teamId,
  title,
  onMenuClick,
  teams,
  organizations,
  effectiveOrgId,
  effectiveTeamId,
  onOrgChange,
  onTeamChange,
}: Props) {
  const navigate = useNavigate();
  const [q, setQ] = useState("");

  function onSearchSubmit(e: FormEvent) {
    e.preventDefault();
    const t = q.trim();
    if (!t || !teamId) return;
    navigate(`/app/teams/${teamId}/search?q=${encodeURIComponent(t)}`);
    setQ("");
  }

  return (
    <header className="sticky top-0 z-30 flex h-14 shrink-0 items-center gap-3 border-b border-slate-200 bg-white px-4 shadow-sm md:px-6">
      {onMenuClick && (
        <button
          type="button"
          onClick={onMenuClick}
          className="rounded-md p-2 text-slate-600 hover:bg-slate-100 md:hidden"
          aria-label="Open menu"
        >
          <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
          </svg>
        </button>
      )}
      <div className="flex min-w-0 flex-1 flex-wrap items-center gap-3">
        {organizations.length > 0 && (
          <label className="flex min-w-0 items-center gap-2 text-sm">
            <span className="hidden text-slate-500 sm:inline">Organization</span>
            <select
              className="dw-select max-w-[160px] sm:max-w-[220px]"
              value={effectiveOrgId}
              onChange={(e) => onOrgChange(e.target.value)}
            >
              {organizations.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.name} ({o.role})
                </option>
              ))}
            </select>
          </label>
        )}
        {teams.length > 0 && (
          <label className="flex min-w-0 items-center gap-2 text-sm">
            <span className="hidden text-slate-500 sm:inline">Team</span>
            <select
              className="dw-select max-w-[200px] sm:max-w-xs"
              value={effectiveTeamId}
              onChange={(e) => onTeamChange(e.target.value)}
            >
              {teams.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name} ({t.role})
                </option>
              ))}
            </select>
          </label>
        )}
        {title && (
          <h1 className="hidden min-w-0 truncate text-lg font-semibold text-slate-900 sm:block">
            {title}
          </h1>
        )}
      </div>
      <form
        onSubmit={onSearchSubmit}
        className="relative hidden sm:block"
        role="search"
      >
        <Search
          className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400"
          strokeWidth={1.75}
        />
        <input
          type="search"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Search projects, apps, servers…"
          aria-label="Search in team"
          className="dw-input w-44 py-2 pl-9 pr-3 xl:w-56"
        />
      </form>
      <UserMenu teamId={teamId} orgId={effectiveOrgId || undefined} />
    </header>
  );
}
