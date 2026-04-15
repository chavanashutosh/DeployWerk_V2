import { useEffect, useState } from "react";
import { apiFetch, resolveApiUrl, type Bootstrap } from "@/api";

type Sys = {
  database_ok: boolean;
  git_sha: string;
};

export function AdminSystemPage() {
  const [s, setS] = useState<Sys | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [boot, setBoot] = useState<Bootstrap | null>(null);

  useEffect(() => {
    let c = false;
    (async () => {
      try {
        const x = await apiFetch<Sys>("/api/v1/admin/system");
        if (!c) {
          setS(x);
          setErr(null);
        }
      } catch (e) {
        if (!c) setErr(e instanceof Error ? e.message : "Failed");
      }
    })();
    return () => {
      c = true;
    };
  }, []);

  useEffect(() => {
    let c = false;
    (async () => {
      try {
        const r = await fetch(resolveApiUrl("/api/v1/bootstrap"));
        if (!r.ok) return;
        const b = (await r.json()) as Bootstrap;
        if (!c) setBoot(b);
      } catch {
        if (!c) setBoot(null);
      }
    })();
    return () => {
      c = true;
    };
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-slate-900">System</h1>
        <p className="mt-1 text-sm text-slate-600">Lightweight health signals for operators.</p>
      </div>
      {err && <p className="text-sm text-red-600">{err}</p>}
      {s && (
        <div className="max-w-lg space-y-4 rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <div>
            <p className="text-xs font-medium uppercase text-slate-500">Database</p>
            <p className="mt-1 text-sm font-medium text-slate-900">
              {s.database_ok ? "Reachable" : "Check failed"}
            </p>
          </div>
          <div>
            <p className="text-xs font-medium uppercase text-slate-500">Build git SHA</p>
            <p className="mt-1 font-mono text-sm text-slate-800">{s.git_sha}</p>
            <p className="mt-2 text-xs text-slate-500">
              Set <code className="rounded bg-slate-100 px-1">DEPLOYWERK_GIT_SHA</code> at compile time for a
              meaningful value.
            </p>
          </div>
        </div>
      )}
      <div className="max-w-2xl space-y-3 rounded-xl border border-slate-200 bg-white p-6 text-sm text-slate-700 shadow-sm">
        <h2 className="text-sm font-semibold text-slate-900">Transactional email (instance)</h2>
        <p className="text-slate-600">
          Team notification endpoints can use an <code className="rounded bg-slate-100 px-1 text-xs">email</code>{" "}
          channel when the API host has SMTP configured.
        </p>
        <ul className="list-inside list-disc space-y-1 text-slate-600">
          <li>
            SMTP:{" "}
            <span className="font-medium text-slate-800">
              {boot?.mail_smtp_configured ? "configured" : "not configured"}
            </span>{" "}
            — set <code className="rounded bg-slate-100 px-1 text-xs">DEPLOYWERK_SMTP_HOST</code> and{" "}
            <code className="rounded bg-slate-100 px-1 text-xs">DEPLOYWERK_SMTP_FROM</code> (see{" "}
            <code className="rounded bg-slate-100 px-1 text-xs">.env.example</code>).
          </li>
          <li>
            Public app URL (invite links):{" "}
            <span className="font-medium text-slate-800">
              {boot?.public_app_url_configured ? "configured" : "not configured"}
            </span>{" "}
            — <code className="rounded bg-slate-100 px-1 text-xs">DEPLOYWERK_PUBLIC_APP_URL</code>.
          </li>
        </ul>
      </div>
      <div className="max-w-2xl space-y-3 rounded-xl border border-slate-200 bg-white p-6 text-sm text-slate-700 shadow-sm">
        <h2 className="text-sm font-semibold text-slate-900">Authentik (optional Docker profile)</h2>
        <p>
          DeployWerk does not ingest Authentik application logs. On the host where Compose runs, stream them with:
        </p>
        <pre className="overflow-x-auto rounded-lg bg-slate-950 p-3 font-mono text-xs text-slate-100">
          docker compose --profile authentik logs -f authentik-server authentik-worker
        </pre>
        {boot?.idp_admin_url && (
          <p className="text-slate-600">
            Authentik admin UI:{" "}
            <a
              href={boot.idp_admin_url}
              className="font-mono text-brand-700 hover:underline"
              target="_blank"
              rel="noreferrer"
            >
              {boot.idp_admin_url}
            </a>
          </p>
        )}
        {boot?.oidc_enabled && boot.authentik_issuer && (
          <p className="text-slate-600">
            OIDC issuer:{" "}
            <a
              href={boot.authentik_issuer}
              className="font-mono text-brand-700 hover:underline"
              target="_blank"
              rel="noreferrer"
            >
              {boot.authentik_issuer}
            </a>
          </p>
        )}
        {!boot?.oidc_enabled && (
          <p className="text-xs text-slate-500">SSO is not enabled on this instance (no OIDC issuer).</p>
        )}
      </div>
    </div>
  );
}
