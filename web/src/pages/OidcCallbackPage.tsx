import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { resolveApiUrl } from "@/api";
import { useAuth } from "@/auth";
import { consumeOidcSession } from "@/oidc";

export function OidcCallbackPage() {
  const { login } = useAuth();
  const nav = useNavigate();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const params = new URLSearchParams(window.location.search);
      const code = params.get("code");
      const sess = consumeOidcSession();
      if (!code || !sess) {
        if (!cancelled) {
          setError("Missing authorization code or session. Start sign-in from the login page.");
        }
        return;
      }
      try {
        const res = await fetch(resolveApiUrl("/api/v1/auth/oidc/callback"), {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            code,
            code_verifier: sess.codeVerifier,
            redirect_uri: sess.redirectUri,
            nonce: sess.nonce,
          }),
        });
        const text = await res.text();
        if (!res.ok) {
          throw new Error(text || res.statusText);
        }
        const data = JSON.parse(text) as { token: string };
        await login(data.token);
        if (!cancelled) nav("/app", { replace: true });
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Sign-in failed");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [login, nav]);

  if (error) {
    return (
      <div className="mx-auto max-w-md px-4 py-16">
        <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
          <h1 className="text-xl font-bold text-slate-900">Could not sign you in</h1>
          <p className="mt-2 text-sm text-red-700">{error}</p>
          <p className="mt-6 text-sm">
            <Link to="/login" className="font-medium text-brand-600 hover:text-brand-700">
              Back to sign in
            </Link>
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-[40vh] items-center justify-center text-slate-600">
      Completing sign-in…
    </div>
  );
}
