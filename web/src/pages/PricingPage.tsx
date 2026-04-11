import { Check } from "lucide-react";

export function PricingPage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-16">
      <h1 className="text-3xl font-bold text-slate-900">Pricing</h1>
      <p className="mt-3 max-w-2xl text-slate-600">
        Hosted / cloud billing (Stripe, plans, invoices) is planned for a later phase. Self-hosted DeployWerk remains
        the primary story for this codebase.
      </p>
      <div className="mt-10 grid gap-8 md:grid-cols-2">
        <div className="rounded-2xl border border-slate-200 bg-white p-8 shadow-sm">
          <h2 className="text-lg font-semibold text-slate-900">Self-hosted</h2>
          <p className="mt-2 text-sm text-slate-600">Run on your infrastructure. No per-seat fee in this edition.</p>
          <ul className="mt-6 space-y-3 text-sm text-slate-700">
            {[
              "Full control of data and network boundaries",
              "Bring your own servers and object storage",
              "Team-scoped resources and API tokens",
            ].map((t) => (
              <li key={t} className="flex gap-2">
                <Check className="mt-0.5 h-4 w-4 shrink-0 text-emerald-600" strokeWidth={1.75} />
                {t}
              </li>
            ))}
          </ul>
        </div>
        <div className="rounded-2xl border border-dashed border-slate-300 bg-slate-50 p-8">
          <h2 className="text-lg font-semibold text-slate-900">Cloud (placeholder)</h2>
          <p className="mt-2 text-sm text-slate-600">
            Subscription tiers, usage limits, and payment webhooks will mirror the reference Part O when implemented.
          </p>
        </div>
      </div>
    </div>
  );
}
