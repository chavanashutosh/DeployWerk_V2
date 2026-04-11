import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { KeyRound, ShieldAlert } from "lucide-react";
import { apiFetch, type Bootstrap } from "@/api";

export function DemoPage() {
  const [data, setData] = useState<Bootstrap | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const b = await apiFetch<Bootstrap>("/api/v1/bootstrap");
        if (!cancelled) setData(b);
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to load");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="mx-auto max-w-3xl px-4 py-16">
      <div className="flex items-start gap-3">
        <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-amber-50 text-amber-800">
          <ShieldAlert className="h-6 w-6" strokeWidth={1.75} />
        </span>
        <div>
          <h1 className="text-3xl font-bold text-slate-900">Sample logins</h1>
          <p className="mt-2 text-slate-600">
            Demo accounts are seeded when the API runs in development and{" "}
            <code className="rounded bg-slate-100 px-1.5 py-0.5 text-sm">DEMO_LOGINS_PUBLIC</code> is enabled (default
            off in production). Passwords are shown here only when the API exposes them.
          </p>
        </div>
      </div>

      {error && (
        <p className="mt-8 rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">{error}</p>
      )}

      {data && !data.demo_logins_enabled && (
        <div className="mt-10 rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
          <p className="text-slate-700">
            Demo credentials are not exposed by this API instance (
            <code className="rounded bg-slate-100 px-1.5 py-0.5 text-sm">demo_logins_enabled: false</code>). Start the
            API with <code className="rounded bg-slate-100 px-1.5 py-0.5 text-sm">APP_ENV=development</code> and see{" "}
            <Link to="/login" className="font-medium text-brand-600">
              Sign in
            </Link>{" "}
            with accounts documented in the README.
          </p>
        </div>
      )}

      {data?.demo_accounts && data.demo_accounts.length > 0 && (
        <div className="mt-10 overflow-hidden rounded-xl border border-slate-200 bg-white shadow-sm">
          <div className="border-b border-slate-100 bg-slate-50 px-6 py-4">
            <h2 className="flex items-center gap-2 text-sm font-semibold text-slate-900">
              <KeyRound className="h-4 w-4" strokeWidth={1.75} />
              Demo team accounts
            </h2>
          </div>
          <table className="min-w-full text-left text-sm">
            <thead>
              <tr className="border-b border-slate-100 text-xs font-semibold uppercase tracking-wide text-slate-500">
                <th className="px-6 py-3">Email</th>
                <th className="px-6 py-3">Role</th>
                <th className="px-6 py-3">Password</th>
              </tr>
            </thead>
            <tbody>
              {data.demo_accounts.map((a) => (
                <tr key={a.email} className="border-b border-slate-50 last:border-0">
                  <td className="px-6 py-3 font-mono text-slate-800">{a.email}</td>
                  <td className="px-6 py-3 text-slate-600">{a.role}</td>
                  <td className="px-6 py-3 font-mono text-slate-800">{a.password}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="border-t border-slate-100 bg-slate-50 px-6 py-4">
            <Link
              to="/login"
              className="text-sm font-semibold text-brand-600 hover:text-brand-700"
            >
              Go to sign in →
            </Link>
          </div>
        </div>
      )}
    </div>
  );
}
