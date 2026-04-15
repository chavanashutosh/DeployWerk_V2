import { useEffect, useState, type FormEvent } from "react";
import { Link, useParams } from "react-router-dom";
import { MailPlus } from "lucide-react";
import { apiFetch, type Bootstrap, type Team } from "@/api";
import { InlineError, PageHeader } from "@/components/ui";

type CreateInvitationResponse = {
  id: string;
  token: string;
  email: string;
  role: string;
  expires_at: string;
  invite_email_sent?: boolean;
};

export function TeamInvitePage() {
  const { teamId = "" } = useParams();
  const [teams, setTeams] = useState<Team[]>([]);
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<"member" | "admin" | "owner">("member");
  const [err, setErr] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [created, setCreated] = useState<CreateInvitationResponse | null>(null);
  const [copied, setCopied] = useState(false);
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);

  const team = teams.find((t) => t.id === teamId);
  const canMutate = team?.role === "admin" || team?.role === "owner";

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [t, b] = await Promise.all([
          apiFetch<Team[]>("/api/v1/teams"),
          apiFetch<Bootstrap>("/api/v1/bootstrap"),
        ]);
        if (!cancelled) {
          setTeams(t);
          setBootstrap(b);
        }
      } catch {
        if (!cancelled) {
          setTeams([]);
          setBootstrap(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    if (!teamId || !email.trim()) return;
    setPending(true);
    setErr(null);
    setCreated(null);
    setCopied(false);
    try {
      const res = await apiFetch<CreateInvitationResponse>(
        `/api/v1/teams/${teamId}/invitations`,
        {
          method: "POST",
          body: JSON.stringify({
            email: email.trim(),
            role,
          }),
        },
      );
      setCreated(res);
      setEmail("");
    } catch (e2) {
      setErr(e2 instanceof Error ? e2.message : "Failed to create invitation");
    } finally {
      setPending(false);
    }
  }

  async function copyLink() {
    if (!created) return;
    const url = `${window.location.origin}/invite/${created.token}`;
    await navigator.clipboard.writeText(url);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  if (!teamId) {
    return <p className="text-slate-600">Missing team.</p>;
  }

  if (!canMutate && teams.length > 0) {
    return (
      <div className="space-y-6">
        <PageHeader
          icon={<MailPlus className="h-6 w-6" strokeWidth={1.75} />}
          title="Invite members"
          description="Only team owners and admins can create invitations."
        />
        <div className="dw-card p-6">
          <Link
            to={`/app/teams/${teamId}/projects`}
            className="text-sm font-semibold text-brand-600 hover:text-brand-700"
          >
            ← Back to projects
          </Link>
        </div>
      </div>
    );
  }

  const inviteUrl =
    created && typeof window !== "undefined"
      ? `${window.location.origin}/invite/${created.token}`
      : "";

  return (
    <div className="space-y-8">
      <PageHeader
        icon={<MailPlus className="h-6 w-6" strokeWidth={1.75} />}
        title="Invite members"
        description={
          <>
            Team <span className="font-medium text-slate-900">{team?.name ?? teamId}</span> ·{" "}
            <Link to={`/app/teams/${teamId}/projects`} className="font-medium text-brand-600 hover:text-brand-700">
              Back to projects
            </Link>
          </>
        }
      />

      <div className="dw-card p-6 sm:p-8">
        {bootstrap && !bootstrap.mail_smtp_configured && (
          <p className="mt-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
            Transactional email is not configured on this instance. Set{" "}
            <code className="rounded bg-white px-1 text-xs">DEPLOYWERK_SMTP_HOST</code> and{" "}
            <code className="rounded bg-white px-1 text-xs">DEPLOYWERK_SMTP_FROM</code> (see{" "}
            <code className="rounded bg-white px-1 text-xs">.env.example</code>) so invites can be emailed
            automatically. You can still copy the link below after creating an invitation.
          </p>
        )}
        {bootstrap?.mail_smtp_configured && !bootstrap.public_app_url_configured && (
          <p className="mt-4 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
            SMTP is configured, but <code className="rounded bg-white px-1 text-xs">DEPLOYWERK_PUBLIC_APP_URL</code> is
            not set. Invite emails need a public app URL for the accept link. Share the copied link manually for now.
          </p>
        )}
        <InlineError message={err} className="mt-4" />
        <form className="mt-6 max-w-md space-y-4" onSubmit={(e) => void onSubmit(e)}>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="inv-email">
              Email
            </label>
            <input
              id="inv-email"
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="dw-input mt-1"
              placeholder="colleague@example.com"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="inv-role">
              Role
            </label>
            <select
              id="inv-role"
              value={role}
              onChange={(e) => setRole(e.target.value as typeof role)}
              className="dw-select mt-1 w-full"
            >
              <option value="member">Member</option>
              <option value="admin">Admin</option>
              <option value="owner">Owner</option>
            </select>
          </div>
          <button type="submit" disabled={pending} className="dw-btn-primary">
            {pending ? "Creating…" : "Create invitation link"}
          </button>
        </form>
      </div>

      {created && (
        <div className="rounded-xl border border-emerald-200 bg-emerald-50/80 p-6 shadow-sm">
          <h2 className="text-lg font-semibold text-slate-900">Invitation ready</h2>
          {created.invite_email_sent === true ? (
            <p className="mt-2 text-sm text-slate-600">
              We sent an email to <span className="font-mono">{created.email}</span> with the invite link (expires{" "}
              {new Date(created.expires_at).toLocaleString()}). You can still copy the link below if needed.
            </p>
          ) : (
            <p className="mt-2 text-sm text-slate-600">
              Share this link with <span className="font-mono">{created.email}</span> (expires{" "}
              {new Date(created.expires_at).toLocaleString()}).{" "}
              {bootstrap?.mail_smtp_configured && bootstrap?.public_app_url_configured
                ? "Email was not sent (hourly invitation limit, SMTP error, or misconfiguration — check API logs)."
                : "Automatic email was not sent (set DEPLOYWERK_SMTP_* and DEPLOYWERK_PUBLIC_APP_URL on the server, or check API logs)."}
            </p>
          )}
          <div className="mt-4 flex flex-wrap items-center gap-2">
            <code className="max-w-full flex-1 break-all rounded-lg bg-white px-3 py-2 text-xs text-slate-800 ring-1 ring-slate-200">
              {inviteUrl}
            </code>
            <button
              type="button"
              onClick={() => void copyLink()}
              className="rounded-lg bg-slate-800 px-4 py-2 text-sm font-semibold text-white hover:bg-slate-900"
            >
              {copied ? "Copied" : "Copy link"}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
