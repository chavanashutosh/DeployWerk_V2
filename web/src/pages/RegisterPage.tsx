import { useEffect, useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useAuth } from "@/auth";
import { apiFetch, type Bootstrap } from "@/api";

export function RegisterPage() {
  const { login } = useAuth();
  const nav = useNavigate();
  const [bootstrap, setBootstrap] = useState<Bootstrap | null>(null);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

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

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setPending(true);
    try {
      const res = await apiFetch<{ token: string }>("/api/v1/auth/register", {
        method: "POST",
        body: JSON.stringify({
          email,
          password,
          name: name.trim() || undefined,
        }),
      });
      await login(res.token);
      nav("/app", { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Registration failed");
    } finally {
      setPending(false);
    }
  }

  const allowLocal = bootstrap?.allow_local_password_auth !== false;

  if (bootstrap && !allowLocal) {
    return (
      <div className="mx-auto max-w-md px-4 py-16">
        <div className="dw-card p-8">
          <h1 className="text-2xl font-semibold tracking-tight text-slate-900">Create account</h1>
          <p className="mt-2 text-sm text-slate-600">
            Local registration is disabled on this instance. Use{" "}
            <Link to="/login" className="dw-link-accent">
              Sign in
            </Link>{" "}
            (SSO).
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-md px-4 py-16">
      <div className="dw-card p-8">
        <h1 className="text-2xl font-semibold tracking-tight text-slate-900">Create account</h1>
        <p className="mt-2 text-sm text-slate-600">
          Already registered?{" "}
          <Link to="/login" className="dw-link-accent">
            Sign in
          </Link>
        </p>
        <form className="mt-8 space-y-4" onSubmit={onSubmit}>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="name">
              Name <span className="font-normal text-slate-400">(optional)</span>
            </label>
            <input
              id="name"
              type="text"
              autoComplete="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="mt-1 dw-input"
            />
          </div>
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
              autoComplete="new-password"
              required
              minLength={8}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="mt-1 dw-input"
            />
            <p className="mt-1 text-xs text-slate-500">At least 8 characters.</p>
          </div>
          {error && (
            <p className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>
          )}
          <button type="submit" disabled={pending} className="dw-btn-primary w-full">
            {pending ? "Creating…" : "Create account"}
          </button>
        </form>
      </div>
    </div>
  );
}
