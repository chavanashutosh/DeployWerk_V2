import { Link, useParams } from "react-router-dom";
import { Inbox, Mail } from "lucide-react";
import { PageHeader } from "@/components/ui";

export function MailOverviewSettingsPage() {
  const { teamId = "" } = useParams();
  const base = `/app/teams/${teamId}/settings`;

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Mail className="h-6 w-6" strokeWidth={1.75} />}
        title="Email & mail"
        description="Configure domains for team mail and notification delivery. Instance operators configure SMTP for transactional email."
      />
      <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-sm font-semibold text-slate-900">Where to go next</h2>
        <ul className="mt-3 list-inside list-disc space-y-2 text-sm text-slate-700">
          <li>
            <Link to={`${base}/mail-domains`} className="font-medium text-brand-700 hover:underline">
              Mail domains
            </Link>{" "}
            — register and validate domains for the team mail platform.
          </li>
          <li>
            <Link to={`${base}/notifications`} className="font-medium text-brand-700 hover:underline">
              Notifications
            </Link>{" "}
            — webhooks and email alerts (email channel requires SMTP on the API host).
          </li>
        </ul>
      </div>
      <div className="flex items-start gap-3 rounded-xl border border-slate-200 bg-slate-50/80 p-6">
        <Inbox className="mt-0.5 h-5 w-5 shrink-0 text-slate-500" strokeWidth={1.75} />
        <div>
          <h2 className="text-sm font-semibold text-slate-900">Webmail (client)</h2>
          <p className="mt-1 text-sm text-slate-600">
            In-browser mail (JMAP) is planned but not shipped yet. See{" "}
            <code className="rounded bg-white px-1 text-xs">docs/spec/08-mail-platform.md</code> in the DeployWerk
            repository for the product direction.
          </p>
        </div>
      </div>
    </div>
  );
}
