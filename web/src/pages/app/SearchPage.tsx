import { useEffect, useMemo, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { apiFetch } from "@/api";

type SearchHit = {
  kind: "project" | "environment" | "application" | "server";
  id: string;
  title: string;
  subtitle?: string | null;
  project_id?: string | null;
  environment_id?: string | null;
};

function hitHref(teamId: string, h: SearchHit): string | null {
  switch (h.kind) {
    case "project":
      if (h.project_id) {
        return `/app/teams/${teamId}/projects/${h.project_id}/environments`;
      }
      return `/app/teams/${teamId}/projects`;
    case "environment":
      if (h.project_id) {
        return `/app/teams/${teamId}/projects/${h.project_id}/environments`;
      }
      return null;
    case "application":
      if (h.project_id && h.environment_id) {
        return `/app/teams/${teamId}/projects/${h.project_id}/environments/${h.environment_id}/applications`;
      }
      return null;
    case "server":
      return `/app/teams/${teamId}/servers`;
    default:
      return null;
  }
}

export function SearchPage() {
  const { teamId = "" } = useParams();
  const [params] = useSearchParams();
  const q = useMemo(() => (params.get("q") ?? "").trim(), [params]);
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    if (!teamId || !q) {
      setHits([]);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const list = await apiFetch<SearchHit[]>(
          `/api/v1/teams/${teamId}/search?q=${encodeURIComponent(q)}`,
        );
        if (!cancelled) {
          setHits(list);
          setErr(null);
        }
      } catch (e) {
        if (!cancelled) {
          setErr(e instanceof Error ? e.message : "Search failed");
          setHits(null);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [teamId, q]);

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-slate-900">Search</h2>
        <p className="mt-1 text-sm text-slate-600">
          {q ? (
            <>
              Results for <span className="font-mono text-slate-800">{q}</span>
            </>
          ) : (
            "Enter a query in the header search box."
          )}
        </p>
      </div>
      {err && <p className="text-sm text-red-600">{err}</p>}
      {hits && hits.length === 0 && q && (
        <p className="rounded-xl border border-slate-200 bg-white p-6 text-sm text-slate-600">
          No matches in projects, environments, applications, or servers.
        </p>
      )}
      {hits && hits.length > 0 && (
        <ul className="divide-y divide-slate-200 rounded-xl border border-slate-200 bg-white shadow-sm">
          {hits.map((h) => {
            const href = hitHref(teamId, h);
            const inner = (
              <>
                <span className="font-medium text-slate-900">{h.title}</span>
                <span className="ml-2 rounded bg-slate-100 px-2 py-0.5 text-xs text-slate-600">
                  {h.kind}
                </span>
                {h.subtitle && (
                  <p className="mt-1 text-sm text-slate-500">{h.subtitle}</p>
                )}
              </>
            );
            return (
              <li key={`${h.kind}-${h.id}`} className="p-4">
                {href ? (
                  <Link to={href} className="block hover:bg-slate-50/80">
                    {inner}
                  </Link>
                ) : (
                  inner
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
