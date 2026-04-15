import type { ReactNode } from "react";
import { useParams } from "react-router-dom";

export function useTeamId() {
  const { teamId = "" } = useParams();
  return teamId;
}

export function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
      <h3 className="text-sm font-semibold text-slate-900">{title}</h3>
      <div className="mt-4">{children}</div>
    </div>
  );
}
