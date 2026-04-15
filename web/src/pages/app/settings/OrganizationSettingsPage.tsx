import { FormEvent, useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import {
  ArrowLeft,
  Building2,
  MailPlus,
  Save,
  Trash2,
  UserCog,
  UserMinus,
} from "lucide-react";
import {
  apiFetch,
  putCurrentOrganization,
  putCurrentTeam,
  type Organization,
  type Team,
} from "@/api";
import { useAuth } from "@/auth";

type OrgMemberRow = {
  user_id: string;
  email: string;
  name: string | null;
  role: "owner" | "admin" | "member";
};

export function OrganizationSettingsPage() {
  const { user, refresh } = useAuth();
  const { orgId = "" } = useParams();
  const navigate = useNavigate();
  const [org, setOrg] = useState<Organization | null>(null);
  const [members, setMembers] = useState<OrgMemberRow[]>([]);
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [mfaRequired, setMfaRequired] = useState(false);
  const [transferTo, setTransferTo] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [backTeamId, setBackTeamId] = useState<string | null>(null);

  useEffect(() => {
    if (!orgId) return;
    let cancelled = false;
    (async () => {
      try {
        const [o, teams] = await Promise.all([
          apiFetch<Organization>(`/api/v1/organizations/${orgId}`),
          apiFetch<Team[]>("/api/v1/teams"),
        ]);
        if (!cancelled) {
          setOrg(o);
          setName(o.name);
          setSlug(o.slug);
          setMfaRequired(Boolean(o.mfa_required));
          const first = teams.find((t) => t.organization_id === orgId);
          setBackTeamId(first?.id ?? teams[0]?.id ?? null);
        }
        const m = await apiFetch<OrgMemberRow[]>(`/api/v1/organizations/${orgId}/members`);
        if (!cancelled) setMembers(m);
        setErr(null);
      } catch (e) {
        if (!cancelled) {
          setOrg(null);
          setMembers([]);
          setErr(e instanceof Error ? e.message : "Failed to load");
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [orgId]);

  const isOwner = org?.role === "owner";
  const canMutate = org?.role === "owner" || org?.role === "admin";

  async function onSaveProfile(e: FormEvent) {
    e.preventDefault();
    if (!orgId) return;
    setPending(true);
    setErr(null);
    try {
      const o = await apiFetch<Organization>(`/api/v1/organizations/${orgId}`, {
        method: "PATCH",
        body: JSON.stringify({
          name: name.trim() || undefined,
          slug: slug.trim() || undefined,
          mfa_required: mfaRequired,
        }),
      });
      setOrg(o);
      setName(o.name);
      setSlug(o.slug);
      setMfaRequired(Boolean(o.mfa_required));
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Save failed");
    } finally {
      setPending(false);
    }
  }

  async function onTransfer(e: FormEvent) {
    e.preventDefault();
    if (!orgId || !transferTo.trim()) return;
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/organizations/${orgId}/transfer-owner`, {
        method: "POST",
        body: JSON.stringify({ new_owner_user_id: transferTo.trim() }),
      });
      setTransferTo("");
      const o = await apiFetch<Organization>(`/api/v1/organizations/${orgId}`);
      setOrg(o);
      const m = await apiFetch<OrgMemberRow[]>(`/api/v1/organizations/${orgId}/members`);
      setMembers(m);
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Transfer failed");
    } finally {
      setPending(false);
    }
  }

  async function reloadMembers() {
    if (!orgId) return;
    const m = await apiFetch<OrgMemberRow[]>(`/api/v1/organizations/${orgId}/members`);
    setMembers(m);
  }

  async function patchOrgMemberRole(uid: string, role: "admin" | "member") {
    if (!orgId) return;
    setErr(null);
    try {
      await apiFetch(`/api/v1/organizations/${orgId}/members/${uid}`, {
        method: "PATCH",
        body: JSON.stringify({ role }),
      });
      await reloadMembers();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Update failed");
    }
  }

  async function removeOrgMember(uid: string) {
    if (!orgId) return;
    if (!window.confirm("Remove this person from the organization? They must have no team memberships left in this org.")) {
      return;
    }
    setErr(null);
    try {
      await apiFetch(`/api/v1/organizations/${orgId}/members/${uid}`, { method: "DELETE" });
      await reloadMembers();
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Remove failed");
    }
  }

  async function onDeleteOrg() {
    if (!orgId) return;
    if (
      !window.confirm(
        `Delete organization “${org?.name ?? orgId}”? All teams must be removed first.`,
      )
    ) {
      return;
    }
    setPending(true);
    setErr(null);
    try {
      await apiFetch(`/api/v1/organizations/${orgId}`, { method: "DELETE" });
      const [teams, orgs] = await Promise.all([
        apiFetch<Team[]>("/api/v1/teams"),
        apiFetch<Organization[]>("/api/v1/organizations"),
      ]);
      const nextOrg = orgs[0];
      const nextTeam = teams[0];
      if (nextOrg && nextTeam) {
        await putCurrentOrganization(nextOrg.id);
        await putCurrentTeam(nextTeam.id);
        await refresh();
        navigate(`/app/teams/${nextTeam.id}/projects`, { replace: true });
      } else {
        await refresh();
        navigate("/app", { replace: true });
      }
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Delete failed");
    } finally {
      setPending(false);
    }
  }

  const settingsBase = backTeamId ? `/app/teams/${backTeamId}/settings` : "/app";

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <div className="flex flex-wrap items-center gap-3">
        <Link
          to={`${settingsBase}/general`}
          className="inline-flex items-center gap-1 text-sm font-medium text-slate-600 hover:text-slate-900"
        >
          <ArrowLeft className="h-4 w-4" strokeWidth={1.75} />
          Back to settings
        </Link>
      </div>

      <div className="flex items-start gap-3 rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
        <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-brand-100 text-brand-800">
          <Building2 className="h-5 w-5" strokeWidth={1.75} />
        </span>
        <div>
          <h1 className="text-xl font-semibold text-slate-900">Organization</h1>
          <p className="mt-1 text-sm text-slate-600">
            Name, slug, and membership for this organization (parent of teams).
          </p>
        </div>
      </div>

      {err && <p className="text-sm text-red-600">{err}</p>}

      {canMutate && org && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <h3 className="text-sm font-semibold text-slate-900">Profile</h3>
          <form onSubmit={onSaveProfile} className="mt-4 space-y-4">
            <label className="block text-sm">
              <span className="text-slate-600">Name</span>
              <input
                className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <label className="block text-sm">
              <span className="text-slate-600">Slug</span>
              <input
                className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
                value={slug}
                onChange={(e) => setSlug(e.target.value)}
              />
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={mfaRequired}
                onChange={(e) => setMfaRequired(e.target.checked)}
              />
              Require MFA for organization members (local-password accounts)
            </label>
            <button
              type="submit"
              disabled={pending}
              className="inline-flex items-center gap-2 rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white disabled:opacity-50"
            >
              <Save className="h-4 w-4" strokeWidth={1.75} />
              Save
            </button>
          </form>
        </div>
      )}

      {!canMutate && org && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <p className="text-sm text-slate-600">
            <span className="font-medium text-slate-900">{org.name}</span>{" "}
            <span className="font-mono text-slate-500">{org.slug}</span> — your role: {org.role}
          </p>
        </div>
      )}

      {canMutate && backTeamId && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <h3 className="flex flex-wrap items-center gap-2 text-sm font-semibold text-slate-900">
            <MailPlus className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
            Add people
          </h3>
          <p className="mt-1 text-sm text-slate-600">
            New members join through a team invitation; accepting adds them to the organization with the right access.
          </p>
          <Link
            to={`/app/teams/${backTeamId}/invite`}
            className="mt-3 inline-flex items-center gap-1.5 rounded-lg border border-slate-200 px-3 py-1.5 text-sm font-medium text-slate-800 hover:bg-slate-50"
          >
            <MailPlus className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
            Invite to team
          </Link>
        </div>
      )}

      {orgId && members.length > 0 && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <h3 className="flex items-center gap-2 text-sm font-semibold text-slate-900">
            <UserCog className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
            Organization members
          </h3>
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
                      value={m.role === "admin" ? "admin" : "member"}
                      onChange={(e) =>
                        void patchOrgMemberRole(m.user_id, e.target.value as "admin" | "member")
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
                      onClick={() => void removeOrgMember(m.user_id)}
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

      {isOwner && (
        <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <h3 className="text-sm font-semibold text-slate-900">Transfer organization ownership</h3>
          <p className="mt-1 text-sm text-slate-600">
            Promote another org member to owner. You become an admin.
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
            Delete the organization only after removing all teams inside it.
          </p>
          <button
            type="button"
            disabled={pending}
            onClick={() => void onDeleteOrg()}
            className="mt-4 inline-flex items-center gap-2 rounded-lg border border-red-300 bg-white px-4 py-2 text-sm font-medium text-red-800 hover:bg-red-50 disabled:opacity-50"
          >
            <Trash2 className="h-4 w-4" strokeWidth={1.75} />
            Delete organization
          </button>
        </div>
      )}
    </div>
  );
}
