import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { Mail } from "lucide-react";
import { apiFetch, type InvitationPublic } from "@/api";
import { useAuth } from "@/auth";
import { InlineError, PageHeader } from "@/components/ui";

export function InvitePage() {
  const { token = "" } = useParams();
  const { user, loading: authLoading } = useAuth();
  const [data, setData] = useState<InvitationPublic | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [acceptErr, setAcceptErr] = useState<string | null>(null);
  const [accepting, setAccepting] = useState(false);

  useEffect(() => {
    if (!token) return;
    let cancelled = false;
    (async () => {
      try {
        const inv = await apiFetch<InvitationPublic>(`/api/v1/invitations/${token}`);
        if (!cancelled) {
          setData(inv);
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load invitation");
          setData(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [token]);

  async function accept() {
    if (!token) return;
    setAccepting(true);
    setAcceptErr(null);
    try {
      await apiFetch(`/api/v1/invitations/${token}/accept`, { method: "POST" });
      window.location.href = "/app";
    } catch (e) {
      setAcceptErr(e instanceof Error ? e.message : "Accept failed");
    } finally {
      setAccepting(false);
    }
  }

  return (
    <div className="mx-auto max-w-lg px-4 py-16">
      <div className="dw-card rounded-2xl p-8 shadow-sm">
        <PageHeader
          icon={<Mail className="h-6 w-6" strokeWidth={1.75} />}
          title="Team invitation"
          description="Accept to join the team with the invited email address."
        />
        <InlineError message={err} className="mt-4" />
        {data && (
          <div className="mt-6 space-y-2 text-sm text-slate-700">
            <p>
              You have been invited to join{" "}
              <span className="font-semibold text-slate-900">{data.team_name}</span> ({data.team_slug}) as{" "}
              <span className="font-mono">{data.role}</span>.
            </p>
            <p>
              Invited email: <span className="font-mono">{data.email}</span>
            </p>
            {data.accepted && <p className="text-emerald-700">This invitation was already accepted.</p>}
            {!data.accepted && (data.expired ?? new Date(data.expires_at) < new Date()) && (
              <p className="text-red-600">This invitation has expired.</p>
            )}
          </div>
        )}
        <InlineError message={acceptErr} className="mt-4" />
        {data &&
          !data.accepted &&
          !(data.expired ?? new Date(data.expires_at) < new Date()) && (
          <div className="mt-8 space-y-3">
            {!user && !authLoading && (
              <p className="text-sm text-slate-600">
                <Link to="/login" className="font-semibold text-brand-600">
                  Sign in
                </Link>{" "}
                with <span className="font-mono">{data.email}</span> to accept.
              </p>
            )}
            {user && (
              <>
                {user.email.toLowerCase() !== data.email.toLowerCase() && (
                  <p className="text-sm text-amber-800">
                    You are signed in as {user.email}. Switch accounts to match the invited email.
                  </p>
                )}
                {user.email.toLowerCase() === data.email.toLowerCase() && (
                  <button
                    type="button"
                    disabled={accepting}
                    onClick={() => void accept()}
                    className="dw-btn-primary"
                  >
                    {accepting ? "Accepting…" : "Accept invitation"}
                  </button>
                )}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
