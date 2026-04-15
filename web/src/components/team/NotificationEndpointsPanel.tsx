import { FormEvent, useEffect, useState } from "react";
import { apiFetch } from "@/api";

export type NotificationEndpoint = {
  id: string;
  team_id: string;
  name: string;
  kind: string;
  target_url?: string | null;
  events: string;
  enabled: boolean;
  created_at: string;
};

export function NotificationEndpointsPanel({ teamId }: { teamId: string }) {
  const [rows, setRows] = useState<NotificationEndpoint[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [kind, setKind] = useState("generic_http");
  const [url, setUrl] = useState("");
  const [events, setEvents] = useState("deploy_succeeded,deploy_failed,deploy_started");

  async function load() {
    if (!teamId) return;
    try {
      const list = await apiFetch<NotificationEndpoint[]>(
        `/api/v1/teams/${teamId}/notification-endpoints`,
      );
      setRows(list);
      setErr(null);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Failed to load");
      setRows(null);
    }
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    try {
      await apiFetch(`/api/v1/teams/${teamId}/notification-endpoints`, {
        method: "POST",
        body: JSON.stringify({ name, kind, target_url: url, events }),
      });
      setName("");
      setUrl("");
      await load();
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : "Create failed");
    }
  }

  async function remove(id: string) {
    if (!teamId) return;
    try {
      await apiFetch(`/api/v1/teams/${teamId}/notification-endpoints/${id}`, {
        method: "DELETE",
      });
      await load();
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : "Delete failed");
    }
  }

  async function test(id: string) {
    if (!teamId) return;
    try {
      const r = await apiFetch<{
        ok: boolean;
        http_status: number;
        channel?: string;
        detail?: string;
      }>(`/api/v1/teams/${teamId}/notification-endpoints/${id}/test`, {
        method: "POST",
        body: "{}",
      });
      const via = r.channel === "smtp" ? "SMTP" : "HTTP";
      const extra = r.detail ? `\n${r.detail}` : "";
      alert(
        r.ok ? `OK (${via}${r.http_status != null ? ` ${r.http_status}` : ""})${extra}` : `Failed (${via} ${r.http_status})${extra}`,
      );
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : "Test failed");
    }
  }

  return (
    <div className="space-y-4">
      {err && <p className="text-sm text-red-600">{err}</p>}
      <form
        onSubmit={onCreate}
        className="grid gap-3 rounded-xl border border-slate-200 bg-slate-50/80 p-4 md:grid-cols-2"
      >
        <div className="md:col-span-2">
          <h3 className="text-sm font-semibold text-slate-900">Add webhook</h3>
          <p className="mt-1 text-xs text-slate-500">
            <code className="rounded bg-white px-1">discord_webhook</code> (Discord embeds),{" "}
            <code className="rounded bg-white px-1">telegram</code> (target{" "}
            <code className="rounded bg-white px-1">CHAT_ID|https://api.telegram.org/bot…/sendMessage</code>
            ), <code className="rounded bg-white px-1">email</code> (recipient address; requires instance SMTP), or{" "}
            <code className="rounded bg-white px-1">generic_http</code> for raw JSON.
          </p>
        </div>
        <label className="text-sm">
          <span className="text-slate-600">Name</span>
          <input
            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
          />
        </label>
        <label className="text-sm">
          <span className="text-slate-600">Kind</span>
          <select
            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
            value={kind}
            onChange={(e) => setKind(e.target.value)}
          >
            <option value="generic_http">generic_http</option>
            <option value="discord_webhook">discord_webhook</option>
            <option value="telegram">telegram</option>
            <option value="email">email</option>
          </select>
        </label>
        <label className="md:col-span-2 text-sm">
          <span className="text-slate-600">Target URL</span>
          <input
            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://…"
            required
          />
        </label>
        <label className="md:col-span-2 text-sm">
          <span className="text-slate-600">Events (comma-separated)</span>
          <input
            className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm"
            value={events}
            onChange={(e) => setEvents(e.target.value)}
          />
        </label>
        <div className="md:col-span-2">
          <button
            type="submit"
            className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white hover:bg-brand-700"
          >
            Create endpoint
          </button>
        </div>
      </form>
      {rows && rows.length === 0 && (
        <p className="text-sm text-slate-600">No notification endpoints yet.</p>
      )}
      {rows && rows.length > 0 && (
        <ul className="divide-y divide-slate-200 rounded-xl border border-slate-200 bg-white">
          {rows.map((r) => (
            <li key={r.id} className="flex flex-wrap items-center justify-between gap-2 p-4">
              <div>
                <p className="font-medium text-slate-900">{r.name}</p>
                <p className="font-mono text-xs text-slate-500">{r.kind}</p>
                <p className="mt-1 text-xs text-slate-400">{r.events}</p>
              </div>
              <div className="flex gap-2">
                <button
                  type="button"
                  className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm text-slate-700 hover:bg-slate-50"
                  onClick={() => void test(r.id)}
                >
                  Test
                </button>
                <button
                  type="button"
                  className="rounded-lg border border-red-200 px-3 py-1.5 text-sm text-red-700 hover:bg-red-50"
                  onClick={() => void remove(r.id)}
                >
                  Delete
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
