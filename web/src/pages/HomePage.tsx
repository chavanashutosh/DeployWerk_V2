import { Link } from "react-router-dom";
import {
  ArrowRight,
  Boxes,
  Cloud,
  GitBranch,
  Shield,
  Terminal,
} from "lucide-react";

const features = [
  {
    title: "Your servers, your rules",
    body: "Connect hosts over SSH, run Docker workloads, and route HTTP with Traefik—without giving up ownership of infrastructure.",
    icon: Cloud,
  },
  {
    title: "Projects & environments",
    body: "Model production, staging, and previews with clear tenancy: organization → team → project → environment → resources.",
    icon: Boxes,
  },
  {
    title: "Git to deploy",
    body: "Build from public or private Git, Dockerfiles, images, or template stacks—aligned with how real teams ship.",
    icon: GitBranch,
  },
  {
    title: "Safe operations",
    body: "Role-aware UI, API tokens with scoped abilities, and background jobs for deploys, backups, and checks.",
    icon: Shield,
  },
  {
    title: "Deep visibility",
    body: "Logs, metrics, health checks, and optional browser terminals—gated by policy you control.",
    icon: Terminal,
  },
];

export function HomePage() {
  return (
    <div>
      <section className="border-b border-slate-200 bg-gradient-to-b from-white to-slate-50">
        <div className="mx-auto max-w-6xl px-4 py-16 md:py-24">
          <p className="text-sm font-semibold uppercase tracking-wide text-brand-600">
            DeployWerk V2 (preview)
          </p>
          <h1 className="mt-3 max-w-3xl text-4xl font-bold tracking-tight text-slate-900 md:text-5xl">
            Self-hosted control plane for applications on your own machines
          </h1>
          <p className="mt-6 max-w-2xl text-lg text-slate-600">
            This repository is an early Rust + web + CLI foundation. Capabilities will grow from the scenario
            document in <code className="rounded bg-slate-100 px-1.5 py-0.5 text-sm">docs/USE_CASES_AND_SCENARIOS.md</code>.
          </p>
          <div className="mt-10 flex flex-wrap gap-3">
            <Link
              to="/register"
              className="inline-flex items-center gap-2 rounded-lg bg-brand-600 px-5 py-3 text-sm font-semibold text-white shadow-sm hover:bg-brand-700"
            >
              Create account
              <ArrowRight className="h-4 w-4" strokeWidth={1.75} />
            </Link>
            <Link
              to="/demo"
              className="inline-flex items-center gap-2 rounded-lg border border-slate-200 bg-white px-5 py-3 text-sm font-semibold text-slate-800 hover:bg-slate-50"
            >
              Sample logins
            </Link>
          </div>
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-4 py-16">
        <h2 className="text-2xl font-bold text-slate-900">What you can expect</h2>
        <p className="mt-2 max-w-2xl text-slate-600">
          The product reference covers tenancy, servers, applications, databases, services, notifications, API access,
          and background automation—implemented incrementally here.
        </p>
        <ul className="mt-10 grid gap-6 md:grid-cols-2">
          {features.map(({ title, body, icon: Icon }) => (
            <li
              key={title}
              className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm transition hover:shadow-md"
            >
              <div className="flex h-11 w-11 items-center justify-center rounded-lg bg-brand-50 text-brand-700">
                <Icon className="h-6 w-6" strokeWidth={1.75} />
              </div>
              <h3 className="mt-4 text-lg font-semibold text-slate-900">{title}</h3>
              <p className="mt-2 text-sm leading-relaxed text-slate-600">{body}</p>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
