export function AdminPricingPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-slate-900">Pricing</h1>
        <p className="mt-1 text-sm text-slate-600">
          The public marketing page at <code className="rounded bg-slate-100 px-1">/pricing</code> is static
          content. Commercial plans, trials, and entitlements are enforced via team{" "}
          <strong>Entitlements</strong> and your billing integration (Stripe, Mollie, etc.).
        </p>
      </div>
      <div className="max-w-2xl space-y-3 rounded-xl border border-slate-200 bg-white p-6 text-sm text-slate-700 shadow-sm">
        <p>
          To require login before viewing <code className="rounded bg-slate-100 px-1">/pricing</code>, set{" "}
          <code className="rounded bg-slate-100 px-1">VITE_PRICING_REQUIRES_AUTH=true</code> when building the web
          app.
        </p>
        <p className="text-slate-600">
          Edit copy in <code className="font-mono text-xs">web/src/pages/PricingPage.tsx</code> for product tiers.
        </p>
      </div>
    </div>
  );
}
