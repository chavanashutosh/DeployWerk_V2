import { FormEvent, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Building2, FolderKanban, Link2, Plus, Rocket, Settings, Sparkles } from "lucide-react";
import { useAuth } from "@/auth";
import { apiFetch, type Organization, type Team } from "@/api";
import { InlineError, PageHeader } from "@/components/ui";

export function DashboardPage() {
  const { user, refresh } = useAuth();
  const [teams, setTeams] = useState<Team[] | null>(null);
  const [organizations, setOrganizations] = useState<Organization[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [newOrgName, setNewOrgName] = useState("");
  const [creatingOrg, setCreatingOrg] = useState(false);

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
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load");
          setTeams(null);
          setOrganizations(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const teamId = user?.current_team_id ?? teams?.[0]?.id;

  const cards =
    teamId ?
      [
        {
          title: "Projects",
          body: "Organize services by project and environment.",
          to: `/app/teams/${teamId}/projects`,
          icon: FolderKanban,
        },
        {
          title: "Deployments",
          body: "Recent Docker deploy jobs across your team.",
          to: `/app/teams/${teamId}/deployments`,
          icon: Rocket,
        },
        {
          title: "Domains",
          body: "Hostnames on applications plus registrar guides.",
          to: `/app/teams/${teamId}/domains`,
          icon: Link2,
        },
        {
          title: "Settings",
          body: "Account, team, organization, tokens, and notifications.",
          to: `/app/teams/${teamId}/settings`,
          icon: Settings,
        },
      ]
    : [];

  async function onCreateOrg(e: FormEvent) {
    e.preventDefault();
    const name = newOrgName.trim();
    if (!name) return;
    setCreatingOrg(true);
    setErr(null);
    try {
      await apiFetch<Organization>("/api/v1/organizations", {
        method: "POST",
        body: JSON.stringify({ name }),
      });
      setNewOrgName("");
      const [t, o] = await Promise.all([
        apiFetch<Team[]>("/api/v1/teams"),
        apiFetch<Organization[]>("/api/v1/organizations"),
      ]);
      setTeams(t);
      setOrganizations(o);
      await refresh();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Could not create organization");
    } finally {
      setCreatingOrg(false);
    }
  }

  const teamsByOrg = new Map<string, Team[]>();
  if (teams) {
    for (const t of teams) {
      const list = teamsByOrg.get(t.organization_id) ?? [];
      list.push(t);
      teamsByOrg.set(t.organization_id, list);
    }
  }

  return (
    <div className="mx-auto max-w-5xl space-y-8">
      <div className="dw-card p-6 sm:p-8">
        <PageHeader
          icon={<Sparkles className="h-6 w-6" strokeWidth={1.75} />}
          title="Overview"
          description={
            <>
              Signed in as <span className="font-medium text-slate-900">{user?.email}</span>. Organizations group one or
              more teams; switch organization or team from the header anytime.
            </>
          }
        />
      </div>

      {teamId && cards.length > 0 && (
        <div>
          <h2 className="text-xs font-semibold uppercase tracking-widest text-slate-500">Quick links</h2>
          <ul className="mt-4 grid gap-4 sm:grid-cols-2">
            {cards.map(({ title, body, to, icon: Icon }) => (
              <li key={to}>
                <Link
                  to={to}
                  className="dw-card flex h-full gap-4 p-5 transition hover:border-slate-300 hover:shadow-md"
                >
                  <Icon className="h-8 w-8 shrink-0 text-slate-700" strokeWidth={1.5} />
                  <div>
                    <p className="font-semibold text-slate-900">{title}</p>
                    <p className="mt-1 text-sm text-slate-600">{body}</p>
                  </div>
                </Link>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="dw-card p-8">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="flex items-start gap-2">
            <Building2 className="mt-0.5 h-5 w-5 text-slate-400" strokeWidth={1.75} />
            <div>
              <h2 className="text-lg font-semibold text-slate-900">Organizations &amp; teams</h2>
              <p className="mt-1 text-sm text-slate-600">
                Each organization contains teams. Open organization settings to rename, transfer ownership, or delete
                (when empty).
              </p>
            </div>
          </div>
        </div>
        <InlineError message={err} className="mt-4" />
        {!err && organizations && organizations.length === 0 && teams && teams.length === 0 && (
          <p className="mt-4 text-sm text-slate-600">You are not a member of any organization yet.</p>
        )}
        {!err && organizations && organizations.length > 0 && (
          <ul className="mt-6 space-y-6">
            {organizations.map((o) => (
              <li key={o.id} className="rounded-xl border border-slate-100 bg-slate-50/50 p-4">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <p className="font-semibold text-slate-900">{o.name}</p>
                    <p className="text-sm text-slate-500">
                      <span className="font-mono">{o.slug}</span> · your role: {o.role}
                    </p>
                  </div>
                  <Link to={`/app/orgs/${o.id}/settings`} className="dw-link-accent text-sm">
                    Organization settings →
                  </Link>
                </div>
                <ul className="mt-3 divide-y divide-slate-200/80 rounded-lg border border-slate-100 bg-white text-sm">
                  {(teamsByOrg.get(o.id) ?? []).map((t) => (
                    <li key={t.id} className="flex flex-wrap items-center justify-between gap-2 px-3 py-2">
                      <span>
                        <span className="font-medium text-slate-800">{t.name}</span>{" "}
                        <span className="font-mono text-xs text-slate-500">{t.slug}</span> · {t.role}
                      </span>
                      <Link to={`/app/teams/${t.id}/projects`} className="dw-link-accent text-sm font-medium">
                        Open →
                      </Link>
                    </li>
                  ))}
                </ul>
                {(teamsByOrg.get(o.id) ?? []).length === 0 && (
                  <p className="mt-2 text-sm text-slate-500">No teams in this organization yet.</p>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="dw-card p-8">
        <h2 className="flex items-center gap-2 text-lg font-semibold text-slate-900">
          <Plus className="h-5 w-5 text-slate-600" strokeWidth={1.75} />
          New organization
        </h2>
        <p className="mt-1 text-sm text-slate-600">
          Creates an empty organization where you are owner. Add teams from team settings or the API.
        </p>
        <form onSubmit={onCreateOrg} className="mt-4 flex flex-wrap items-end gap-2">
          <label className="min-w-[200px] flex-1 text-sm">
            <span className="text-xs font-medium uppercase tracking-wider text-slate-500">Name</span>
            <input
              className="mt-1 dw-input"
              value={newOrgName}
              onChange={(e) => setNewOrgName(e.target.value)}
              placeholder="e.g. Acme Corp"
            />
          </label>
          <button
            type="submit"
            disabled={creatingOrg || !newOrgName.trim()}
            className="dw-btn-primary gap-2"
          >
            <Plus className="h-4 w-4" strokeWidth={1.75} />
            Create
          </button>
        </form>
      </div>
    </div>
  );
}
