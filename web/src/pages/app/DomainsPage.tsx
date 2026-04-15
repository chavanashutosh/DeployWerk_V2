import { useEffect, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { ExternalLink } from "lucide-react";
import { apiFetch, type Bootstrap, type Team, type TeamDomainRow } from "@/api";

const registrars = [
  {
    name: "Cloudflare Registrar",
    href: "https://www.cloudflare.com/products/registrar/",
    blurb: "At-cost pricing with DNS and CDN adjacent.",
  },
  {
    name: "Namecheap",
    href: "https://www.namecheap.com/domains/",
    blurb: "Popular retail registrar; easy DNS templates.",
  },
];

export function DomainsPage() {
  const { teamId = "" } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const tab = searchParams.get("tab") || "overview";
  const [rows, setRows] = useState<TeamDomainRow[] | null>(null);
  const [teams, setTeams] = useState<Team[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);

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

  useEffect(() => {
    if (!teamId) return;
    let cancelled = false;
    (async () => {
      try {
        const list = await apiFetch<TeamDomainRow[]>(`/api/v1/teams/${teamId}/domains`);
        if (!cancelled) {
          setRows(list);
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Failed to load");
          setRows(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId]);

  function setTab(t: string) {
    setSearchParams({ tab: t });
  }

  const team = teams.find((t) => t.id === teamId);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-slate-900">Domains</h2>
        <p className="mt-1 text-sm text-slate-600">
          Hostnames attached to applications in <span className="font-medium">{team?.name ?? "this team"}</span>.
          DeployWerk does not sell domains in this preview—use a registrar, then attach hostnames on each application.
        </p>
        {bootstrap?.apps_base_domain && (
          <p className="mt-2 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900">
            New applications receive a random hostname under{" "}
            <span className="font-mono font-medium">*.{bootstrap.apps_base_domain}</span> until you change the
            application&apos;s domain list. Point wildcard DNS for that zone at your edge (e.g. Traefik) when using{" "}
            <code className="rounded bg-white px-1">DEPLOYWERK_EDGE_MODE=traefik</code>.
          </p>
        )}
      </div>

      <div className="flex flex-wrap gap-2 border-b border-slate-200 pb-2">
        {(["overview", "register", "transfer"] as const).map((t) => (
          <button
            key={t}
            type="button"
            onClick={() => setTab(t)}
            className={`rounded-lg px-3 py-1.5 text-sm font-medium ${
              tab === t ? "bg-brand-100 text-brand-800" : "text-slate-600 hover:bg-slate-100"
            }`}
          >
            {t === "overview" ? "Attached domains" : t === "register" ? "Buy a domain" : "Transfer in"}
          </button>
        ))}
      </div>

      {tab === "overview" && (
        <>
          {err && <p className="text-sm text-red-600">{err}</p>}
          {rows && rows.length === 0 && (
            <p className="rounded-xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
              No domain strings on applications yet. Add domains in the application editor, or start with{" "}
              <button type="button" className="font-medium text-brand-600" onClick={() => setTab("register")}>
                Buy a domain
              </button>
              .
            </p>
          )}
          {rows && rows.length > 0 && (
            <div className="overflow-hidden rounded-xl border border-slate-200 bg-white shadow-sm">
              <table className="min-w-full text-left text-sm">
                <thead>
                  <tr className="border-b border-slate-100 bg-slate-50 text-xs font-semibold uppercase tracking-wide text-slate-500">
                    <th className="px-4 py-3">Domain</th>
                    <th className="px-4 py-3">Source</th>
                    <th className="px-4 py-3">Application</th>
                    <th className="px-4 py-3">Environment</th>
                    <th className="px-4 py-3">Project</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r) => (
                    <tr key={`${r.domain}-${r.application_id}`} className="border-b border-slate-50 last:border-0">
                      <td className="px-4 py-3 font-mono text-slate-900">{r.domain}</td>
                      <td className="px-4 py-3 text-slate-600">
                        {r.provisioned ? (
                          <span className="rounded-full bg-sky-100 px-2 py-0.5 text-xs font-medium text-sky-800">
                            Provisioned
                          </span>
                        ) : (
                          <span className="text-xs text-slate-500">Custom</span>
                        )}
                      </td>
                      <td className="px-4 py-3">{r.application_name}</td>
                      <td className="px-4 py-3 text-slate-600">{r.environment_name}</td>
                      <td className="px-4 py-3 text-slate-600">{r.project_name}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}

      {tab === "register" && (
        <div className="space-y-6 rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <div className="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
            <strong>Preview only.</strong> DeployWerk does not process payments or register domains. Complete purchase on
            a registrar site, then add the hostname to your application’s domain list in the app settings.
          </div>
          <div>
            <label className="text-sm font-medium text-slate-700" htmlFor="q">
              Search a name (illustration)
            </label>
            <input
              id="q"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="your-product.com"
              className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
            />
            <p className="mt-2 text-xs text-slate-500">
              Availability checks require a registrar API—use your preferred site to search and buy.
            </p>
          </div>
          <div>
            <h3 className="text-sm font-semibold text-slate-900">Choose a registrar</h3>
            <ul className="mt-3 grid gap-3 sm:grid-cols-2">
              {registrars.map((r) => (
                <li key={r.name}>
                  <a
                    href={r.href}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex h-full flex-col rounded-xl border border-slate-200 p-4 transition hover:border-brand-300 hover:bg-slate-50"
                  >
                    <span className="flex items-center gap-1 font-medium text-brand-700">
                      {r.name}
                      <ExternalLink className="h-3.5 w-3.5" strokeWidth={1.75} />
                    </span>
                    <span className="mt-1 text-sm text-slate-600">{r.blurb}</span>
                  </a>
                </li>
              ))}
            </ul>
          </div>
          <div>
            <h3 className="text-sm font-semibold text-slate-900">After you own the domain</h3>
            <ol className="mt-2 list-decimal space-y-1 pl-5 text-sm text-slate-600">
              <li>Create DNS records at your registrar (A/AAAA or CNAME to your server or future edge).</li>
              <li>Add the hostname to the application in DeployWerk so the control plane tracks intent.</li>
              <li>When Traefik / TLS automation ships, point DNS to the documented target.</li>
            </ol>
          </div>
          <p className="text-sm">
            <Link to={`/app/teams/${teamId}/projects`} className="font-medium text-brand-600 hover:text-brand-700">
              Go to projects →
            </Link>
          </p>
        </div>
      )}

      {tab === "transfer" && (
        <div className="space-y-4 rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <p className="text-sm text-slate-600">
            Transferring a domain between registrars is handled entirely at your provider. Use this checklist before you
            move DNS to infrastructure managed by DeployWerk.
          </p>
          <ul className="list-disc space-y-2 pl-5 text-sm text-slate-700">
            <li>Unlock the domain at the losing registrar.</li>
            <li>Request an authorization (EPP) code.</li>
            <li>Disable WHOIS privacy if it blocks transfer emails.</li>
            <li>Initiate transfer at the gaining registrar; approve emails promptly.</li>
            <li>After transfer, update nameservers or A/CNAME records to your DeployWerk targets.</li>
          </ul>
          <div className="rounded-lg border border-slate-100 bg-slate-50 p-4 text-sm text-slate-600">
            Need help? See{" "}
            <a
              href="https://www.icann.org/resources/pages/transfer-en"
              target="_blank"
              rel="noopener noreferrer"
              className="font-medium text-brand-600"
            >
              ICANN transfer policy
            </a>{" "}
            and your registrar’s docs.
          </div>
        </div>
      )}
    </div>
  );
}
