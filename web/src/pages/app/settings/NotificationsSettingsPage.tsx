import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { apiFetch, type Bootstrap } from "@/api";
import { NotificationEndpointsPanel } from "@/components/team/NotificationEndpointsPanel";

export function NotificationsSettingsPage() {
  const { teamId = "" } = useParams();
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const b = await apiFetch<Bootstrap>("/api/v1/bootstrap");
        if (!cancelled) setBootstrap(b);
      } catch {
        if (!cancelled) setBootstrap(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-slate-900">Notifications</h2>
        <p className="mt-1 text-sm text-slate-600">
          Webhooks and optional email fire on deploy lifecycle events (started, succeeded, failed). Use Discord,
          generic HTTP URLs, or an <code className="rounded bg-slate-100 px-1 text-xs">email</code> recipient when the
          instance has SMTP configured.
        </p>
        {bootstrap && !bootstrap.mail_smtp_configured && (
          <p className="mt-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
            Email notification endpoints are disabled until operators set{" "}
            <code className="rounded bg-white px-1 text-xs">DEPLOYWERK_SMTP_HOST</code> and{" "}
            <code className="rounded bg-white px-1 text-xs">DEPLOYWERK_SMTP_FROM</code> on the API host.
          </p>
        )}
      </div>
      <NotificationEndpointsPanel teamId={teamId} />
    </div>
  );
}
