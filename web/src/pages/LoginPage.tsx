import { useEffect, useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { KeyRound } from "lucide-react";
import { useAuth } from "@/auth";
import { apiFetch, type Bootstrap, type OidcConfig } from "@/api";
import { beginAuthentikLogin } from "@/oidc";

export function LoginPage() {
  const { login } = useAuth();
  const nav = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [totp, setTotp] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [oidc, setOidc] = useState<OidcConfig | null>(null);
  const [demoLoading, setDemoLoading] = useState<string | null>(null);
  const [oidcBusy, setOidcBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [b, o] = await Promise.all([
          apiFetch<Bootstrap>("/api/v1/bootstrap"),
          apiFetch<OidcConfig>("/api/v1/auth/oidc/config"),
        ]);
        if (!cancelled) {
          setBootstrap(b);
          setOidc(o);
        }
      } catch {
        if (!cancelled) {
          setBootstrap(null);
          setOidc(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setPending(true);
    try {
      const res = await apiFetch<{ token: string }>("/api/v1/auth/login", {
        method: "POST",
        body: JSON.stringify({ email, password, totp_code: totp.trim() || undefined }),
      });
      await login(res.token);
      nav("/app", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setPending(false);
    }
  }

  async function signInAs(demoEmail: string, demoPassword: string) {
    setError(null);
    setDemoLoading(demoEmail);
    try {
      const res = await apiFetch<{ token: string }>("/api/v1/auth/login", {
        method: "POST",
        body: JSON.stringify({ email: demoEmail, password: demoPassword }),
      });
      await login(res.token);
      nav("/app", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Demo login failed");
    } finally {
      setDemoLoading(null);
    }
  }

  const demos = bootstrap?.demo_accounts ?? [];
  const allowLocal = bootstrap?.allow_local_password_auth !== false;
  const oidcEnabled = oidc?.enabled === true && !!oidc.authorization_endpoint && !!oidc.client_id;

  async function onAuthentik() {
    if (!oidc?.authorization_endpoint || !oidc.client_id) return;
    setError(null);
    setOidcBusy(true);
    try {
      await beginAuthentikLogin({
        authorizationEndpoint: oidc.authorization_endpoint,
        clientId: oidc.client_id,
        scopes: oidc.scopes ?? "openid profile email",
        apiRedirectUri: oidc.redirect_uri ?? null,
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not start SSO");
      setOidcBusy(false);
    }
  }

  return (
    <div className="mx-auto max-w-md px-4 py-16">
      <div className="dw-card p-8">
        <h1 className="text-2xl font-semibold tracking-tight text-slate-900">Sign in</h1>
        <p className="mt-2 text-sm text-slate-600">
          {allowLocal ? (
            <>
              No account?{" "}
              <Link to="/register" className="dw-link-accent">
                Register
              </Link>
            </>
          ) : (
            <span className="text-slate-500">Registration is disabled. Use your organization sign-in.</span>
          )}
        </p>

        {oidcEnabled && (
          <div className="mt-6">
            <button
              type="button"
              disabled={oidcBusy}
              onClick={() => void onAuthentik()}
              className="dw-btn-secondary w-full"
            >
              {oidcBusy ? "Redirecting…" : "Continue with Authentik"}
            </button>
          </div>
        )}

        {bootstrap?.idp_admin_url && (
          <p className={`text-center text-xs text-slate-500 ${oidcEnabled ? "mt-3" : "mt-6"}`}>
            <a
              href={bootstrap.idp_admin_url}
              target="_blank"
              rel="noreferrer"
              className="dw-link text-xs"
            >
              Open IdP admin
            </a>{" "}
            (Authentik: OAuth providers, users, flows)
          </p>
        )}

        {allowLocal && (
        <form className="mt-8 space-y-4" onSubmit={onSubmit}>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="email">
              Email
            </label>
            <input
              id="email"
              type="email"
              autoComplete="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="mt-1 dw-input"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="password">
              Password
            </label>
            <input
              id="password"
              type="password"
              autoComplete="current-password"
              required
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="mt-1 dw-input"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="totp">
              MFA code (if required)
            </label>
            <input
              id="totp"
              inputMode="numeric"
              value={totp}
              onChange={(e) => setTotp(e.target.value)}
              className="mt-1 dw-input"
              placeholder="123456"
            />
            <p className="mt-1 text-xs text-slate-500">
              If your organization requires MFA, enroll under your account and enter your 6-digit TOTP code here.
            </p>
          </div>
          {error && (
            <p className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
          )}
          <button type="submit" disabled={pending} className="dw-btn-primary w-full">
            {pending ? "Signing in…" : "Sign in"}
          </button>
        </form>
        )}

        {demos.length > 0 && allowLocal && (
          <div className="mt-8 border-t border-slate-200 pt-6">
            <h2 className="flex items-center gap-2 text-sm font-semibold text-slate-900">
              <KeyRound className="h-4 w-4 text-slate-600" strokeWidth={1.75} />
              Sample logins (development)
            </h2>
            <p className="mt-1 text-xs text-slate-500">
              One-click sign-in for seeded demo users. Not available when the API hides demo passwords.
            </p>
            <ul className="mt-3 space-y-2">
              {demos.map((a) => (
                <li key={a.email}>
                  <button
                    type="button"
                    disabled={!!demoLoading}
                    onClick={() => void signInAs(a.email, a.password)}
                    className="w-full rounded-md border border-slate-200 bg-slate-50 px-3 py-2 text-left text-sm transition hover:border-slate-300 hover:bg-white disabled:opacity-50"
                  >
                    <span className="font-medium text-slate-900">{a.role}</span>
                    <span className="mt-0.5 block font-mono text-xs text-slate-600">{a.email}</span>
                    {demoLoading === a.email ? (
                      <span className="mt-1 block text-xs text-slate-600">Signing in…</span>
                    ) : (
                      <span className="mt-1 block text-xs text-slate-600">Click to sign in →</span>
                    )}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}

        {bootstrap && !bootstrap.demo_logins_enabled && (
          <p className="mt-6 text-center text-xs text-slate-500">
            Demo quick-login is off for this API. Use{" "}
            <Link to="/demo" className="dw-link-accent text-xs">
              Sample logins
            </Link>{" "}
            for documentation, or credentials from the README.
          </p>
        )}

        <p className="mt-6 text-center text-sm text-slate-500">
          <Link to="/demo" className="dw-link-accent text-sm">
            Full demo table →
          </Link>
        </p>
      </div>
    </div>
  );
}
