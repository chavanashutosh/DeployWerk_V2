import { Link, useParams } from "react-router-dom";

export function SettingsDomainsPage() {
  const { teamId = "" } = useParams();
  return (
    <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
      <h3 className="text-sm font-semibold text-slate-900">Domains &amp; DNS</h3>
      <p className="mt-2 text-sm text-slate-600">
        Manage hostnames attached to applications, or walk through buying and transferring domains with external
        registrars.
      </p>
      <div className="mt-4 flex flex-wrap gap-3">
        <Link
          to={`/app/teams/${teamId}/domains`}
          className="inline-flex rounded-lg bg-brand-600 px-4 py-2 text-sm font-semibold text-white hover:bg-brand-700"
        >
          Open domains hub
        </Link>
        <Link
          to={`/app/teams/${teamId}/domains?tab=register`}
          className="inline-flex rounded-lg border border-slate-200 px-4 py-2 text-sm font-medium text-slate-800 hover:bg-slate-50"
        >
          Buy a domain (guide)
        </Link>
      </div>
    </div>
  );
}
