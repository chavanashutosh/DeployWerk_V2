import { FormEvent, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import {
  MailPlus,
  Plus,
  Save,
  Trash2,
  UserMinus,
} from "lucide-react";
import {
  apiFetch,
  putCurrentTeam,
  type Organization,
  type Team,
  type TeamInvitationRow,
} from "@/api";
import { useAuth } from "@/auth";

type TeamMemberRow = {
  user_id: string;
  email: string;
  name: string | null;
  role: "owner" | "admin" | "member";
};

export function TeamSettingsPage() {
  const { user, refresh } = useAuth();
  const { teamId = "" } = useParams();
  const navigate = useNavigate();
  const [team, setTeam] = useState<Team | null>(null);
  const [teamName, setTeamName] = useState("");
  const [teamSlug, setTeamSlug] = useState("");
  const [org, setOrg] = useState<Organization | null>(null);
  const [members, setMembers] = useState<TeamMemberRow[]>([]);
  const [invites, setInvites] = useState<TeamInvitationRow[]>([]);
  const [transferTo, setTransferTo] = useState("");
  const [newTeamName, setNewTeamName] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  async function reload() {
    if (!teamId) return;
    const teams = await apiFetch<Team[]>("/api/v1/teams");
    const t = teams.find((x) => x.id === teamId) ?? null;
    setTeam(t);
    if (t?.organization_id) {
      try {
        const o = await apiFetch<Organization>(`/api/v1/organizations/${t.organization_id}`);
        setOrg(o);
      } catch {
        setOrg(null);
      }
    } else {
      setOrg(null);
    }
    const m = await apiFetch<TeamMemberRow[]>(`/api/v1/teams/${teamId}/members`);
    setMembers(m);
    try {
      const inv = await apiFetch<TeamInvitationRow[]>(`/api/v1/teams/${teamId}/invitations`);
      setInvites(inv);
    } catch {
      setInvites([]);
    }
  }

  useEffect(() => {
    if (!teamId) return;
    let cancelled = false;
    (async () => {
      try {
        await reload();
        if (!cancelled) setErr(null);
      } catch (e) {
        if (!cancelled) {
          setTeam(null);
          setOrg(null);
          setMembers([]);
          setInvites([]);
          setErr(e instanceof Error ? e.message : "Failed to load");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId]);

  useEffect(() => {
    if (!team) return;
    setTeamName(team.name);
    setTeamSlug(team.slug);
  }, [team?.id, team?.name, team?.slug]);

  const isOwner = team?.role === "owner";
  const canMutate = team?.role === "admin" || team?.role === "owner";
  const canOrgMutate = org?.role === "admin" || org?.role === "owner";

  async function onSaveTeamProfile(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch<Team>(`/api/v1/teams/${teamId}`, {
        method: "PATCH",
        body: JSON.stringify({
          name: teamName.trim() || undefined,
          slug: teamSlug.trim() || undefined,
        }),
      });
      await refresh();
      await reload();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Save failed");
    } finally {
      setPending(false);
    }
  }

  async function onTransfer(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !transferTo.trim()) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/transfer-owner`, {
        method: "POST",
        body: JSON.stringify({ new_owner_user_id: transferTo.trim() }),
      });
      setTransferTo("");
      await reload();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Transfer failed");
    } finally {
      setPending(false);
    }
  }

  async function onDeleteTeam() {
    if (!teamId) return;
    if (
      !window.confirm(
        `Delete team “${team?.name ?? teamId}” and all projects, environments, and data? This cannot be undone.`,
      )
    ) {
      return;
    }
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}`, { method: "DELETE" });
      const teams = await apiFetch<Team[]>("/api/v1/teams");
      const next = teams[0];
      if (next) {
        await putCurrentTeam(next.id);
        navigate(`/app/teams/${next.id}/projects`, { replace: true });
      } else {
        navigate("/app", { replace: true });
      }
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Delete failed");
    } finally {
      setPending(false);
    }
  }

  async function revokeInvite(id: string) {
    if (!teamId || !window.confirm("Revoke this invitation?")) return;
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/invitations/${id}`, { method: "DELETE" });
      await reload();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Revoke failed");
    }
  }

  async function patchMemberRole(uid: string, role: "admin" | "member") {
    if (!teamId) return;
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/members/${uid}`, {
        method: "PATCH",
        body: JSON.stringify({ role }),
      });
      await reload();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Update failed");
    }
  }

  async function removeMember(uid: string) {
    if (!teamId) return;
    if (!window.confirm("Remove this member from the team?")) return;
    setErr(null);
    try {
      await apiFetch(`/api/v1/teams/${teamId}/members/${uid}`, { method: "DELETE" });
      await reload();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Remove failed");
    }
  }

  async function createAnotherTeam(e: FormEvent) {
    e.preventDefault();
    if (!team?.organization_id || !newTeamName.trim()) return;
    setPending(true);
    setErr(null);
    try {
      const created = await apiFetch<Team>(
        `/api/v1/organizations/${team.organization_id}/teams`,
        {
          method: "POST",
          body: JSON.stringify({ name: newTeamName.trim() }),
        },
      );
      setNewTeamName("");
      await putCurrentTeam(created.id);
      await refresh();
      navigate(`/app/teams/${created.id}/projects`, { replace: true });
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Could not create team");
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h3 className="text-sm font-semibold text-slate-900">Team</h3>
        {team ? (
          <>
            {canMutate ? (
              <form onSubmit={onSaveTeamProfile} className="mt-4 space-y-4">
                <label className="block text-sm">
                  <span className="text-xs text-slate-500">Name</span>
                  <input
                    className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                    value={teamName}
                    onChange={(e) => setTeamName(e.target.value)}
                    required
                  />
                </label>
                <label className="block text-sm">
                  <span className="text-xs text-slate-500">Slug</span>
                  <input
                    className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                    value={teamSlug}
                    onChange={(e) => setTeamSlug(e.target.value)}
                    required
                  />
                </label>
                <p className="text-sm text-slate-600">Your role: {team.role}</p>
                {team.organization_id && (
                  <p className="text-xs text-slate-500">
                    Organization ID: <span className="font-mono">{team.organization_id}</span>
                  </p>
                )}
                <button
                  type="submit"
                  disabled={pending || !teamName.trim() || !teamSlug.trim()}
                  className="inline-flex items-center gap-2 rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
                >
                  <Save className="h-4 w-4" strokeWidth={1.75} />
                  Save
                </button>
              </form>
            ) : (
              <>
                <p className="mt-2 text-lg font-medium text-slate-800">{team.name}</p>
                <p className="mt-1 font-mono text-sm text-slate-500">{team.slug}</p>
                <p className="mt-1 text-sm text-slate-600">Your role: {team.role}</p>
                {team.organization_id && (
                  <p className="mt-2 text-xs text-slate-500">
                    Organization ID: <span className="font-mono">{team.organization_id}</span>
                  </p>
                )}
              </>
            )}
          </>
        ) : (
          <p className="mt-2 text-sm text-slate-500">Could not load team.</p>
        )}
        {err && <p className="mt-3 text-sm text-red-600">{err}</p>}
      </div>

      {canMutate && teamId && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-sm font-semibold text-slate-900">Invitations</h3>
            <Link
              to={`/app/teams/${teamId}/invite`}
              className="inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-3 py-1.5 text-sm font-medium text-slate-800 hover:bg-slate-50"
            >
              <MailPlus className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
              Invite
            </Link>
          </div>
          <ul className="mt-3 divide-y divide-slate-100 text-sm">
            {invites
              .filter((i) => !i.accepted)
              .map((i) => (
                <li key={i.id} className="flex flex-wrap items-center justify-between gap-2 py-2">
                  <span>
                    <span className="font-medium text-slate-800">{i.email}</span>{" "}
                    <span className="text-slate-500">({i.role})</span>
                    <span className="ml-2 text-xs text-slate-400">
                      expires {new Date(i.expires_at).toLocaleDateString()}
                    </span>
                  </span>
                  <button
                    type="button"
                    onClick={() => void revokeInvite(i.id)}
                    className="inline-flex items-center gap-1 rounded border border-red-200 px-2 py-1 text-xs font-medium text-red-700 hover:bg-red-50"
                  >
                    <Trash2 className="h-3.5 w-3.5" strokeWidth={1.75} />
                    Revoke
                  </button>
                </li>
              ))}
          </ul>
          {invites.filter((i) => !i.accepted).length === 0 && (
            <p className="mt-2 text-sm text-slate-500">No pending invitations.</p>
          )}
        </div>
      )}

      {teamId && members.length > 0 && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <h3 className="text-sm font-semibold text-slate-900">Members</h3>
          <ul className="mt-3 divide-y divide-slate-100 text-sm">
            {members.map((m) => (
              <li key={m.user_id} className="flex flex-wrap items-center justify-between gap-2 py-3">
                <span>
                  <span className="font-medium text-slate-800">{m.email}</span>
                  {m.name && <span className="text-slate-500"> — {m.name}</span>}
                </span>
                <div className="flex flex-wrap items-center gap-2">
                  {canMutate && m.role !== "owner" ? (
                    <select
                      className="rounded-lg border border-slate-200 px-2 py-1 text-xs"
                      value={m.role}
                      onChange={(e) =>
                        void patchMemberRole(m.user_id, e.target.value as "admin" | "member")
                      }
                    >
                      <option value="admin">admin</option>
                      <option value="member">member</option>
                    </select>
                  ) : (
                    <span className="text-slate-500">{m.role}</span>
                  )}
                  {canMutate && m.role !== "owner" && m.user_id !== user?.id ? (
                    <button
                      type="button"
                      onClick={() => void removeMember(m.user_id)}
                      className="inline-flex items-center gap-1 rounded border border-red-200 px-2 py-1 text-xs font-medium text-red-700 hover:bg-red-50"
                    >
                      <UserMinus className="h-3.5 w-3.5" strokeWidth={1.75} />
                      Remove
                    </button>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {canOrgMutate && team?.organization_id && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <h3 className="flex items-center gap-2 text-sm font-semibold text-slate-900">
            <Plus className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
            New team in organization
          </h3>
          <p className="mt-1 text-sm text-slate-600">
            Create another team under the same organization. You will be the team owner.
          </p>
          <form onSubmit={createAnotherTeam} className="mt-4 flex flex-wrap items-end gap-2">
            <label className="min-w-[200px] flex-1 text-sm">
              <span className="text-xs text-slate-500">Team name</span>
              <input
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={newTeamName}
                onChange={(e) => setNewTeamName(e.target.value)}
                placeholder="e.g. Platform"
              />
            </label>
            <button
              type="submit"
              disabled={pending || !newTeamName.trim()}
              className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
            >
              Create
            </button>
          </form>
        </div>
      )}

      {isOwner && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <h3 className="text-sm font-semibold text-slate-900">Transfer ownership</h3>
          <p className="mt-1 text-sm text-slate-600">
            Promote another member to owner. You will become an admin.
          </p>
          <form onSubmit={onTransfer} className="mt-4 flex flex-wrap items-end gap-2">
            <label className="text-sm">
              <span className="block text-xs text-slate-500">New owner</span>
              <select
                className="mt-1 rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={transferTo}
                onChange={(e) => setTransferTo(e.target.value)}
                required
              >
                <option value="">Select member</option>
                {members
                  .filter((m) => m.role !== "owner")
                  .map((m) => (
                    <option key={m.user_id} value={m.user_id}>
                      {m.email}
                    </option>
                  ))}
              </select>
            </label>
            <button
              type="submit"
              disabled={pending || !transferTo}
              className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
            >
              Transfer
            </button>
          </form>
        </div>
      )}

      {isOwner && (
        <div className="rounded-xl border border-red-200 bg-red-50/40 p-6 shadow-sm">
          <h3 className="text-sm font-semibold text-red-900">Danger zone</h3>
          <p className="mt-1 text-sm text-red-800/90">
            Delete this team permanently. All projects, applications, and related records are removed.
          </p>
          <button
            type="button"
            disabled={pending}
            onClick={() => void onDeleteTeam()}
            className="mt-4 inline-flex items-center gap-2 rounded-lg border border-red-300 bg-white px-4 py-2 text-sm font-medium text-red-800 hover:bg-red-50 disabled:opacity-50"
          >
            <Trash2 className="h-4 w-4" strokeWidth={1.75} />
            Delete team
          </button>
        </div>
      )}
    </div>
  );
}
