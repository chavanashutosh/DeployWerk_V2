import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Copy, ExternalLink } from "lucide-react";
import { apiFetch, resolveApiUrl, type Bootstrap, type PlatformIntegrationsBootstrap } from "@/api";
import { useAuth } from "@/auth";
import { InlineError } from "@/components/ui";

/** Stable labels for future i18n extraction. */
const CARD_DEFS: {
  id: string;
  title: string;
  description: string;
  urlKey: keyof PlatformIntegrationsBootstrap;
}[] = [
  {
    id: "forgejo",
    title: "Forgejo (Git)",
    description: "Repositories, merge requests, and GitLab-style deploy webhooks.",
    urlKey: "forgejoUrl",
  },
  {
    id: "mailcow",
    title: "Mailcow",
    description: "Mailboxes, SOGo, and SMTP submission for transactional email.",
    urlKey: "mailcowUrl",
  },
  {
    id: "portainer",
    title: "Portainer",
    description: "Containers, stacks, and host Docker operations.",
    urlKey: "portainerUrl",
  },
  {
    id: "technitium",
    title: "Technitium DNS",
    description: "Authoritative DNS for your domains.",
    urlKey: "technitiumUrl",
  },
  {
    id: "matrix",
    title: "Matrix client",
    description: "Chat / Element or another Matrix client URL.",
    urlKey: "matrixClientUrl",
  },
  {
    id: "traefik",
    title: "Traefik",
    description: "Edge routing and TLS (dashboard if exposed).",
    urlKey: "traefikDashboardUrl",
  },
];

function pickUrl(
  pi: PlatformIntegrationsBootstrap | undefined,
  key: keyof PlatformIntegrationsBootstrap,
): string | undefined {
  const v = pi?.[key];
  return typeof v === "string" && v.length > 0 ? v : undefined;
}

type Props = {
  teamId: string | undefined;
};

export function PlatformIntegrationsPanel({ teamId }: Props) {
  const { user } = useAuth();
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [probeMsg, setProbeMsg] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let c = false;
    (async () => {
      try {
        const b = await apiFetch<Bootstrap>("/api/v1/bootstrap");
        if (!c) setBoot(b);
      } catch {
        if (!c) setBoot(null);
      }
    })();
    return () => {
      c = true;
    };
  }, []);

  const pi = boot?.platform_integrations;
  const publicBase =
    boot?.public_app_url?.replace(/\/+$/, "") ||
    (typeof window !== "undefined" ? window.location.origin : "");
  const webhookUrl = teamId
    ? `${publicBase}/api/v1/hooks/gitlab/${teamId}`
    : "";
  async function copyWebhook() {
    if (!webhookUrl) return;
    try {
      await navigator.clipboard.writeText(webhookUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopied(false);
    }
  }

  async function probePortainer() {
    setProbeMsg(null);
    setErr(null);
    try {
      const j = await apiFetch<{ ok?: boolean; portainer?: unknown }>(
        "/api/v1/admin/integrations/portainer/health",
      );
      setProbeMsg(j.ok ? "Portainer API responded successfully." : JSON.stringify(j));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Probe failed (integration may be disabled).");
    }
  }

  async function probeTechnitium() {
    setProbeMsg(null);
    setErr(null);
    try {
      const j = await apiFetch<{ ok?: boolean; note?: string }>(
        "/api/v1/admin/integrations/technitium/status",
      );
      setProbeMsg(j.note || (j.ok ? "Technitium integration configured." : "Unknown response."));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Probe failed (integration may be disabled).");
    }
  }

  const isAdmin = user?.is_platform_admin === true;

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-sm font-semibold text-slate-900">Platform services</h3>
        {pi?.localServiceDefaults ? (
          <p className="mt-1 text-sm text-slate-600">
            Localhost preset is <strong>on</strong> (<code className="rounded bg-slate-100 px-1 text-xs">DEPLOYWERK_LOCAL_SERVICE_DEFAULTS</code>
            ). Override any link with a matching <code className="rounded bg-slate-100 px-1 text-xs">DEPLOYWERK_INTEGRATION_*</code> URL on the API.
          </p>
        ) : (
          <p className="mt-1 text-sm text-slate-600">
            Quick setup on the same machine as DeployWerk: set{" "}
            <code className="rounded bg-slate-100 px-1 text-xs">DEPLOYWERK_LOCAL_SERVICE_DEFAULTS=true</code> on the API, or set individual{" "}
            <code className="rounded bg-slate-100 px-1 text-xs">DEPLOYWERK_INTEGRATION_*</code> URLs.
          </p>
        )}
      </div>

      <ul className="grid gap-3 sm:grid-cols-2">
        {CARD_DEFS.map((c) => {
          const href = pickUrl(pi, c.urlKey);
          return (
            <li key={c.id} className="dw-card flex flex-col rounded-xl border border-slate-200 p-4">
              <div className="flex-1">
                <h4 className="font-medium text-slate-900">{c.title}</h4>
                <p className="mt-1 text-xs text-slate-600">{c.description}</p>
              </div>
              {href ? (
                <a
                  href={href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="mt-3 inline-flex items-center gap-1.5 text-sm font-medium text-brand-700 hover:underline"
                >
                  Open
                  <ExternalLink className="h-3.5 w-3.5" aria-hidden />
                </a>
              ) : (
                <p className="mt-3 text-xs text-slate-400">Not configured</p>
              )}
            </li>
          );
        })}
      </ul>

      {teamId && webhookUrl ? (
        <div className="dw-card rounded-xl border border-slate-200 p-4">
          <h4 className="text-sm font-semibold text-slate-900">Forgejo / GitLab webhook (this team)</h4>
          <p className="mt-1 text-xs text-slate-600">
            Push events: POST to the URL below. See README for signing options.
          </p>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <code className="max-w-full break-all rounded bg-slate-100 px-2 py-1 text-xs text-slate-800">{webhookUrl}</code>
            <button
              type="button"
              onClick={() => void copyWebhook()}
              className="inline-flex items-center gap-1 rounded-lg border border-slate-200 px-2 py-1 text-xs text-slate-700 hover:bg-slate-50"
            >
              <Copy className="h-3.5 w-3.5" />
              {copied ? "Copied" : "Copy"}
            </button>
          </div>
          {pickUrl(pi, "forgejoUrl") ? (
            <p className="mt-2 text-xs text-slate-500">
              Clone base: <span className="font-mono">{pickUrl(pi, "forgejoUrl")}</span>
            </p>
          ) : null}
        </div>
      ) : null}

      <div className="dw-card rounded-xl border border-slate-200 p-4">
        <h4 className="text-sm font-semibold text-slate-900">Single sign-on</h4>
        <p className="mt-1 text-xs text-slate-600">
          OIDC status:{" "}
          <strong>{boot?.oidc_enabled ? "enabled" : "not configured"}</strong>
          {boot?.authentik_issuer ? (
            <>
              {" "}
              — issuer <code className="rounded bg-slate-100 px-1 text-[11px]">{boot.authentik_issuer}</code>
            </>
          ) : null}
        </p>
        <p className="mt-2">
          {pi?.ssoPlaybookUrl?.startsWith("http") ? (
            <a
              href={pi.ssoPlaybookUrl}
              className="text-sm font-medium text-brand-700 hover:underline"
              target="_blank"
              rel="noopener noreferrer"
            >
              SSO docs (repository README)
            </a>
          ) : (
            <Link to="/app/sso-setup" className="text-sm font-medium text-brand-700 hover:underline">
              Open in-app SSO guide
            </Link>
          )}
        </p>
      </div>

      {isAdmin ? (
        <details className="dw-card rounded-xl border border-slate-200 p-4">
          <summary className="cursor-pointer text-sm font-semibold text-slate-900">
            Advanced — API probes (optional)
          </summary>
          <p className="mt-2 text-xs text-slate-600">
            Requires <code className="rounded bg-slate-100 px-1">DEPLOYWERK_PORTAINER_INTEGRATION_*</code> or Technitium DNS env on the API.
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => void probePortainer()}
              className="rounded-lg bg-slate-900 px-3 py-1.5 text-xs text-white hover:bg-slate-800"
            >
              Test Portainer API
            </button>
            <button
              type="button"
              onClick={() => void probeTechnitium()}
              className="rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-xs text-slate-800 hover:bg-slate-50"
            >
              Technitium status
            </button>
          </div>
          {probeMsg ? <p className="mt-2 text-xs text-slate-700">{probeMsg}</p> : null}
          {err ? <InlineError className="mt-2" message={err} /> : null}
        </details>
      ) : null}

      <p className="text-xs text-slate-500">
        API base for this UI: <code className="rounded bg-slate-100 px-1">{resolveApiUrl("/api")}</code>
      </p>
    </div>
  );
}
