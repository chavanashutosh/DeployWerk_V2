import { ArrowLeft, Eye, Save, Settings2 } from "lucide-react";
import { FormEvent, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { apiFetch } from "@/api";
import { toastError, toastSuccess } from "@/toast";
import { AdminTableWrap, AdminThead, AdminViewLink, formatAdminListError } from "./adminUi";

type Detail = {
  team: {
    id: string;
    organization_id: string;
    org_name: string;
    name: string;
    slug: string;
    created_at: string;
  };
  members: { user_id: string; email: string; name: string | null; role: string }[];
  billing: {
    team_id: string;
    plan_name: string;
    status: string;
    payment_provider: string;
    provider_customer_id: string | null;
    stripe_customer_id?: string | null;
    updated_at: string;
  } | null;
};

type BillingEvent = {
  id: string;
  event_code: string;
  psp_reference: string | null;
  merchant_reference: string | null;
  created_at: string;
};

export function AdminTeamDetailPage() {
  const { teamId = "" } = useParams();
  const [d, setD] = useState<Detail | null>(null);
  const [events, setEvents] = useState<BillingEvent[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [plan_name, setPlan] = useState("");
  const [status, setStatus] = useState("");
  const [payment_provider, setPayProv] = useState("none");
  const [provider_customer_id, setProvId] = useState("");
  const [stripe_customer_id, setStripe] = useState("");

  async function load() {
    if (!teamId) return;
    try {
      const x = await apiFetch<Detail>(`/api/v1/admin/teams/${teamId}`);
      setD(x);
      if (x.billing) {
        setPlan(x.billing.plan_name);
        setStatus(x.billing.status);
        setPayProv(x.billing.payment_provider || "none");
        setProvId(x.billing.provider_customer_id ?? "");
        setStripe(x.billing.stripe_customer_id ?? "");
      } else {
        setPlan("free");
        setStatus("inactive");
        setPayProv("none");
        setProvId("");
        setStripe("");
      }
      const ev = await apiFetch<BillingEvent[]>(`/api/v1/admin/billing/${teamId}/events?limit=50`);
      setEvents(ev);
      setErr(null);
    } catch (e) {
      setErr(formatAdminListError(e));
    }
  }

  useEffect(() => {
    void load();
  }, [teamId]);

  async function saveBilling(e: FormEvent) {
    e.preventDefault();
    if (!teamId) return;
    try {
      await apiFetch(`/api/v1/admin/billing/${teamId}`, {
        method: "PATCH",
        body: JSON.stringify({
          plan_name,
          status,
          payment_provider: payment_provider || "none",
          provider_customer_id: provider_customer_id || null,
          stripe_customer_id: stripe_customer_id || null,
        }),
      });
      toastSuccess("Billing updated");
      await load();
    } catch (ex) {
      const m = formatAdminListError(ex);
      setErr(m);
      toastError(m);
    }
  }

  if (!d && !err) return <p className="text-slate-600">Loading…</p>;
  if (err && !d)
    return (
      <p className="text-red-600">
        {err}{" "}
        <Link to="/admin/teams" className="inline-flex items-center gap-1 text-violet-700 hover:underline">
          <ArrowLeft className="h-4 w-4" strokeWidth={1.75} aria-hidden />
          Teams
        </Link>
      </p>
    );
  if (!d) return null;

  return (
    <div className="space-y-8">
      <div>
        <Link to="/admin/teams" className="inline-flex items-center gap-1 text-sm text-violet-700 hover:underline">
          <ArrowLeft className="h-4 w-4" strokeWidth={1.75} aria-hidden />
          Teams
        </Link>
        <h1 className="mt-2 text-2xl font-semibold text-slate-900">{d.team.name}</h1>
        <p className="mt-1 text-sm text-slate-600">
          {d.team.slug} · org:{" "}
          <AdminViewLink
            to={`/admin/organizations/${d.team.organization_id}`}
            label={d.team.org_name}
            icon={Eye}
          />
        </p>
        <Link
          className="mt-3 inline-flex items-center gap-2 text-sm font-medium text-violet-700 hover:text-violet-900 hover:underline"
          to={`/admin/teams/${teamId}/entitlements`}
        >
          <Settings2 className="h-4 w-4 shrink-0" strokeWidth={1.75} aria-hidden />
          Manage entitlements
        </Link>
      </div>

      {err && <p className="text-sm text-red-600">{err}</p>}

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Members</h2>
        <ul className="mt-3 space-y-2 text-sm">
          {d.members.map((m) => (
            <li key={m.user_id}>
              <AdminViewLink to={`/admin/users/${m.user_id}`} label={m.email} icon={Eye} />{" "}
              <span className="text-slate-500">({m.role})</span>
            </li>
          ))}
        </ul>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Billing (admin)</h2>
        <p className="mt-1 text-sm text-slate-600">
          Overrides team billing row. Use Adyen <code className="rounded bg-slate-100 px-1">merchantReference</code>{" "}
          team UUID or <code className="rounded bg-slate-100 px-1">deploywerk_team_{"{uuid}"}</code> for webhooks.
        </p>
        <form onSubmit={saveBilling} className="mt-4 max-w-lg space-y-3">
          <label className="block text-sm">
            <span className="text-slate-600">Plan</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
              value={plan_name}
              onChange={(e) => setPlan(e.target.value)}
            />
          </label>
          <label className="block text-sm">
            <span className="text-slate-600">Status</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
              value={status}
              onChange={(e) => setStatus(e.target.value)}
            />
          </label>
          <label className="block text-sm">
            <span className="text-slate-600">Payment provider</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
              value={payment_provider}
              onChange={(e) => setPayProv(e.target.value)}
              placeholder="none | adyen | stripe"
            />
          </label>
          <label className="block text-sm">
            <span className="text-slate-600">Provider customer ID</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
              value={provider_customer_id}
              onChange={(e) => setProvId(e.target.value)}
            />
          </label>
          <label className="block text-sm">
            <span className="text-slate-600">Stripe customer ID (legacy)</span>
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 font-mono text-sm focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/20"
              value={stripe_customer_id}
              onChange={(e) => setStripe(e.target.value)}
            />
          </label>
          <button
            type="submit"
            className="inline-flex items-center gap-2 rounded-lg bg-violet-600 px-4 py-2 text-sm font-medium text-white hover:bg-violet-700"
          >
            <Save className="h-4 w-4" strokeWidth={1.75} aria-hidden />
            Save billing
          </button>
        </form>
      </section>

      <section className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h2 className="text-lg font-semibold text-slate-900">Billing events</h2>
        <p className="mt-1 text-sm text-slate-600">Recent provider notifications (e.g. Adyen).</p>
        <AdminTableWrap className="mt-4 max-h-64">
          <table className="w-full text-sm">
            <AdminThead>
              <tr>
                <th className="px-4 py-2 font-medium">Time</th>
                <th className="px-4 py-2 font-medium">Code</th>
                <th className="px-4 py-2 font-medium">PSP ref</th>
              </tr>
            </AdminThead>
            <tbody>
              {events.map((ev) => (
                <tr key={ev.id} className="border-b border-slate-100">
                  <td className="px-4 py-2 text-slate-600">{new Date(ev.created_at).toLocaleString()}</td>
                  <td className="px-4 py-2">{ev.event_code}</td>
                  <td className="px-4 py-2 font-mono text-xs text-slate-600">{ev.psp_reference ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {events.length === 0 && <p className="px-4 py-6 text-sm text-slate-500">No events yet.</p>}
        </AdminTableWrap>
      </section>
    </div>
  );
}
