import { useEffect, useMemo, useState } from "react";
import { Outlet, useLocation, useNavigate, useParams } from "react-router-dom";
import { apiFetch, type Organization, type Team, type User } from "@/api";
import type { AppShellOutletContext } from "@/appShellContext";
import { useAuth } from "@/auth";
import { SidebarNav } from "@/components/dashboard/SidebarNav";
import { TopHeader } from "@/components/dashboard/TopHeader";

const titleFromPath: { prefix: string; title: string }[] = [
  { prefix: "/applications", title: "Applications" },
  { prefix: "/environments", title: "Environments" },
  { prefix: "/projects", title: "Projects" },
  { prefix: "/deployments", title: "Deployments" },
  { prefix: "/logs", title: "Logs" },
  { prefix: "/servers", title: "Servers" },
  { prefix: "/docker", title: "Docker" },
  { prefix: "/destinations", title: "Destinations" },
  { prefix: "/domains", title: "Domains" },
  { prefix: "/analytics", title: "Analytics" },
  { prefix: "/speed-insights", title: "Speed Insights" },
  { prefix: "/observability", title: "Observability" },
  { prefix: "/firewall", title: "Firewall" },
  { prefix: "/cdn", title: "CDN" },
  { prefix: "/integrations", title: "Integrations" },
  { prefix: "/storage", title: "Storage" },
  { prefix: "/flags", title: "Feature flags" },
  { prefix: "/agent", title: "Agent" },
  { prefix: "/ai-gateway", title: "AI Gateway" },
  { prefix: "/sandboxes", title: "Sandboxes" },
  { prefix: "/usage", title: "Usage" },
  { prefix: "/support", title: "Support" },
  { prefix: "/cli", title: "Web CLI" },
  { prefix: "/search", title: "Search" },
  { prefix: "/settings", title: "Settings" },
  { prefix: "/invite", title: "Invite members" },
];

function headerTitle(pathname: string): string | undefined {
  if (pathname === "/app" || pathname.endsWith("/app")) return "Overview";
  const teamIdx = pathname.indexOf("/teams/");
  if (teamIdx === -1) return undefined;
  const rest = pathname.slice(teamIdx + "/teams/".length);
  const slash = rest.indexOf("/");
  const sub = slash === -1 ? "" : rest.slice(slash);
  for (const { prefix, title } of titleFromPath) {
    if (sub.includes(prefix)) return title;
  }
  return "Dashboard";
}

export function AppShell() {
  const { user, setCurrentTeam, setCurrentOrganization } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const params = useParams();
  const teamIdFromRoute = params.teamId;
  const [teams, setTeams] = useState<Team[]>([]);
  const [organizations, setOrganizations] = useState<Organization[]>([]);
  const [mobileNav, setMobileNav] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [t, o] = await Promise.all([
          apiFetch<Team[]>("/api/v1/teams"),
          apiFetch<Organization[]>("/api/v1/organizations"),
        ]);
        if (!cancelled) {
          setTeams(t);
          setOrganizations(o);
        }
      } catch {
        if (!cancelled) {
          setTeams([]);
          setOrganizations([]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [user?.current_team_id, user?.current_organization_id]);

  useEffect(() => {
    if (!user || teams.length === 0) return;
    if (user.current_team_id) return;
    if (teamIdFromRoute) return;
    void setCurrentTeam(teams[0].id);
  }, [user, teams, teamIdFromRoute, setCurrentTeam]);

  const effectiveTeamId =
    teamIdFromRoute ?? user?.current_team_id ?? teams[0]?.id ?? "";

  const currentTeam = teams.find((t) => t.id === effectiveTeamId);
  const effectiveOrgId =
    currentTeam?.organization_id ??
    user?.current_organization_id ??
    organizations[0]?.id ??
    "";

  const teamsInOrg = teams.filter((t) => t.organization_id === effectiveOrgId);

  const canInvite =
    currentTeam?.role === "admin" ||
    currentTeam?.role === "owner" ||
    !!(effectiveOrgId && user?.organization_admin_organization_ids?.includes(effectiveOrgId));

  const headerT = useMemo(() => headerTitle(location.pathname), [location.pathname]);

  async function onTeamChange(nextId: string) {
    if (!nextId) return;
    try {
      await setCurrentTeam(nextId);
      const { pathname } = location;
      if (pathname.startsWith("/app/teams/")) {
        const nextPath = pathname.replace(/\/app\/teams\/[^/]+/, `/app/teams/${nextId}`);
        navigate(nextPath);
      } else {
        navigate(`/app/teams/${nextId}/projects`, { replace: true });
      }
    } catch {
      /* ignore */
    }
  }

  async function onOrgChange(nextOrgId: string) {
    if (!nextOrgId) return;
    try {
      await setCurrentOrganization(nextOrgId);
      const me = await apiFetch<User>("/api/v1/me");
      const [t, o] = await Promise.all([
        apiFetch<Team[]>("/api/v1/teams"),
        apiFetch<Organization[]>("/api/v1/organizations"),
      ]);
      setTeams(t);
      setOrganizations(o);
      const tid =
        me.current_team_id ?? t.find((x) => x.organization_id === nextOrgId)?.id ?? "";
      if (!tid) return;
      const { pathname } = location;
      if (pathname.startsWith("/app/teams/")) {
        navigate(pathname.replace(/\/app\/teams\/[^/]+/, `/app/teams/${tid}`));
      } else {
        navigate(`/app/teams/${tid}/projects`, { replace: true });
      }
    } catch {
      /* ignore */
    }
  }

  useEffect(() => {
    setMobileNav(false);
  }, [location.pathname]);

  const menuTeamId = effectiveTeamId || teams[0]?.id || "";

  return (
    <div className="flex h-dvh overflow-hidden bg-slate-50">
      {/* Desktop sidebar */}
      <aside className="hidden h-dvh w-60 shrink-0 flex-col border-r border-slate-800 bg-slate-950 md:flex">
        <div className="flex h-14 shrink-0 items-center border-b border-slate-800 px-4">
          <span className="text-sm font-semibold tracking-tight text-white">DeployWerk</span>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          {menuTeamId ? (
            <SidebarNav teamId={menuTeamId} canInvite={canInvite} />
          ) : (
            <p className="p-4 text-sm text-slate-400">Loading teams…</p>
          )}
        </div>
      </aside>

      {/* Mobile drawer */}
      <div
        className={`fixed inset-0 z-40 bg-black/40 transition-opacity md:hidden ${
          mobileNav ? "opacity-100" : "pointer-events-none opacity-0"
        }`}
        aria-hidden={!mobileNav}
        onClick={() => setMobileNav(false)}
      />
      <aside
        className={`fixed inset-y-0 left-0 z-50 flex h-dvh w-60 transform flex-col border-r border-slate-800 bg-slate-950 shadow-xl transition-transform md:hidden ${
          mobileNav ? "translate-x-0" : "-translate-x-full"
        }`}
      >
        <div className="flex h-14 shrink-0 items-center justify-between border-b border-slate-800 px-4">
          <span className="text-sm font-semibold text-white">Menu</span>
          <button
            type="button"
            className="rounded-md p-2 text-slate-400 hover:bg-slate-800 hover:text-white"
            onClick={() => setMobileNav(false)}
            aria-label="Close menu"
          >
            ✕
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto pb-8">
          {menuTeamId ? (
            <SidebarNav
              teamId={menuTeamId}
              canInvite={canInvite}
              onNavigate={() => setMobileNav(false)}
            />
          ) : null}
        </div>
      </aside>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {menuTeamId ? (
          <TopHeader
            teamId={menuTeamId}
            title={headerT}
            teams={teamsInOrg.length > 0 ? teamsInOrg : teams}
            organizations={organizations}
            effectiveOrgId={effectiveOrgId}
            effectiveTeamId={effectiveTeamId || menuTeamId}
            onOrgChange={(id) => void onOrgChange(id)}
            onTeamChange={(id) => void onTeamChange(id)}
            onMenuClick={() => setMobileNav(true)}
          />
        ) : (
          <div className="flex h-14 items-center border-b border-slate-200 bg-white px-4 text-sm text-slate-600 shadow-sm">
            DeployWerk
          </div>
        )}
        <main className="min-h-0 flex-1 overflow-y-auto p-4 md:p-6">
          <Outlet
            context={{ teams, organizations } satisfies AppShellOutletContext}
          />
        </main>
      </div>
    </div>
  );
}
