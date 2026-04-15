import { Link } from "react-router-dom";
import { ArrowRight, Check } from "lucide-react";

export function PricingPage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-16">
      <p className="text-sm font-semibold uppercase tracking-wide text-brand-600">Pricing</p>
      <h1 className="mt-2 text-3xl font-bold text-slate-900">Pricing that fits how you run</h1>
      <p className="mt-4 max-w-2xl text-lg text-slate-600">
        A Vercel- or Netlify-style workflow for your team, without mandatory vendor hosting: you operate the API,
        database, and web UI—predictable boundaries, no surprise egress from us. A future managed SKU may add metered
        usage; today the focus is transparent self-hosting and a dashboard that matches the roadmap.
      </p>
      <div className="mt-10 grid gap-8 md:grid-cols-2">
        <div className="dw-card rounded-2xl p-8 shadow-sm">
          <h2 className="text-lg font-semibold text-slate-900">Self-hosted (today)</h2>
          <p className="mt-2 text-sm text-slate-600">
            Run API, Postgres, and dashboard where you want—ideal when you need the same deploy UX as cloud platforms
            but custody of data and network stays yours.
          </p>
          <ul className="mt-6 space-y-3 text-sm text-slate-700">
            {[
              "Full control of data and network boundaries",
              "Bring your own servers and object storage",
              "Team-scoped resources, API tokens, and deploy jobs",
            ].map((t) => (
              <li key={t} className="flex gap-2">
                <Check className="mt-0.5 h-4 w-4 shrink-0 text-emerald-600" strokeWidth={1.75} />
                {t}
              </li>
            ))}
          </ul>
          <Link
            to="/register"
            className="dw-btn-primary mt-8 inline-flex gap-2 px-5 py-2.5 text-sm"
          >
            Get started
            <ArrowRight className="h-4 w-4" strokeWidth={1.75} />
          </Link>
        </div>
        <div className="dw-card rounded-2xl border-dashed border-slate-300 bg-gradient-to-br from-brand-50/30 to-white p-8">
          <h2 className="text-lg font-semibold text-slate-900">Cloud (roadmap)</h2>
          <p className="mt-2 text-sm text-slate-600">
            Subscription tiers, Stripe billing, and seat limits will follow the reference doc (Part O) when a managed
            SKU is ready. Nothing here blocks you from shipping on your own metal today.
          </p>
          <p className="mt-6 text-sm text-slate-500">
            Questions? Start a conversation in your issue tracker or internal chat—we designed the dashboard nav to mirror
            the long-term product so pricing conversations map cleanly to features.
          </p>
          <Link to="/login" className="mt-4 inline-block text-sm font-semibold text-brand-600 hover:text-brand-700">
            Sign in to the dashboard →
          </Link>
        </div>
      </div>
    </div>
  );
}
