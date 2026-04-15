import { Link } from "react-router-dom";
import { PageHeader } from "@/components/ui";

/**
 * Operator SSO guide (Phase 2). Labels grouped for future i18n.
 * @see README.md § Single sign-on (OIDC) (duplicate content for repo browsers).
 */
export function SsoPlaybookPage() {
  return (
    <div className="mx-auto max-w-3xl space-y-8">
      <PageHeader
        title="Single sign-on playbook"
        description="Align DeployWerk with your identity provider and other self-hosted apps (Forgejo, Portainer, etc.)."
      />
      <article className="dw-card space-y-6 rounded-xl p-6 text-sm leading-relaxed text-slate-700">
        <section className="space-y-2">
          <h2 className="text-base font-semibold text-slate-900">1. Choose an IdP hub</h2>
          <p>
            Use one OpenID Connect provider (e.g. Authentik, Keycloak) as the source of truth for human users. DeployWerk
            reads <code className="rounded bg-slate-100 px-1">AUTHENTIK_ISSUER</code>,{" "}
            <code className="rounded bg-slate-100 px-1">AUTHENTIK_CLIENT_ID</code>, and{" "}
            <code className="rounded bg-slate-100 px-1">AUTHENTIK_CLIENT_SECRET</code> — see{" "}
            <code className="rounded bg-slate-100 px-1">.env.example</code>.
          </p>
        </section>
        <section className="space-y-2">
          <h2 className="text-base font-semibold text-slate-900">2. DeployWerk OAuth2 application</h2>
          <p>
            In your IdP, create an OAuth2/OIDC application. Redirect URI must match{" "}
            <code className="rounded bg-slate-100 px-1">AUTHENTIK_REDIRECT_URI</code> (e.g.{" "}
            <code className="rounded bg-slate-100 px-1">https://your-domain/login/oidc/callback</code>).
          </p>
        </section>
        <section className="space-y-2">
          <h2 className="text-base font-semibold text-slate-900">3. Forgejo</h2>
          <p>
            In Forgejo: Site administration → Authentication Sources → Add OAuth2 (OpenID Connect). Use the same IdP
            issuer and client where Forgejo supports it; redirect URIs must match Forgejo&apos;s callback URL.
          </p>
        </section>
        <section className="space-y-2">
          <h2 className="text-base font-semibold text-slate-900">4. Portainer</h2>
          <p>
            Portainer supports OAuth; configure it with your IdP&apos;s authorization and token endpoints. Keep API
            access tokens separate from interactive login when using{" "}
            <code className="rounded bg-slate-100 px-1">DEPLOYWERK_PORTAINER_INTEGRATION_ENABLED</code>.
          </p>
        </section>
        <section className="space-y-2">
          <h2 className="text-base font-semibold text-slate-900">5. SCIM (optional)</h2>
          <p>
            If you provision users from Authentik via SCIM, set <code className="rounded bg-slate-100 px-1">DEPLOYWERK_SCIM_BEARER_TOKEN</code>{" "}
            and related vars per <code className="rounded bg-slate-100 px-1">.env.example</code>.
          </p>
        </section>
        <p className="text-slate-500">
          <Link to="/app" className="text-brand-700 hover:underline">
            Back to app
          </Link>
        </p>
      </article>
    </div>
  );
}
