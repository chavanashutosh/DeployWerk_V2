import { useEffect, useState } from "react";
import { useAuth } from "@/auth";
import { apiFetch, type Team } from "@/api";

export function DashboardPage() {
  const { user } = useAuth();
  const [teams, setTeams] = useState<Team[] | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const t = await apiFetch<Team[]>("/api/v1/teams");
        if (!cancelled) setTeams(t);
      } catch (e) {
        if (!cancelled) setErr(e instanceof Error ? e.message : "Failed to load teams");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="space-y-8">
      <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
        <h1 className="text-2xl font-bold text-slate-900">Overview</h1>
        <p className="mt-2 text-slate-600">
          Signed in as <span className="font-medium text-slate-900">{user?.email}</span>. This dashboard will expand
          with projects, environments, and resources per{" "}
          <code className="rounded bg-slate-100 px-1.5 py-0.5 text-sm">docs/USE_CASES_AND_SCENARIOS.md</code>.
        </p>
      </div>

      <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Your teams</h2>
        {err && <p className="mt-4 text-sm text-red-600">{err}</p>}
        {!err && teams && teams.length === 0 && (
          <p className="mt-4 text-sm text-slate-600">You are not a member of any team yet.</p>
        )}
        {!err && teams && teams.length > 0 && (
          <ul className="mt-4 divide-y divide-slate-100">
            {teams.map((t) => (
              <li key={t.id} className="flex flex-wrap items-center justify-between gap-2 py-4 first:pt-0">
                <div>
                  <p className="font-medium text-slate-900">{t.name}</p>
                  <p className="text-sm text-slate-500">
                    <span className="font-mono">{t.slug}</span> · {t.role}
                  </p>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
