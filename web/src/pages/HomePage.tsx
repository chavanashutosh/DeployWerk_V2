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
    title: "From Git to running apps",
    body: "Push, build, and deploy with a workflow that feels familiar if you have used Vercel or Netlify—projects, environments, and deploy history in one dashboard.",
    icon: GitBranch,
  },
  {
    title: "Projects and environments",
    body: "Organize production, staging, and previews the way your team already thinks: organization, team, project, environment, then applications.",
    icon: Boxes,
  },
  {
    title: "Your compute, your network",
    body: "Run on SSH hosts and Docker you control, with optional Traefik-style routing—no mandatory multi-tenant runtime.",
    icon: Cloud,
  },
  {
    title: "Team-ready from day one",
    body: "Roles, invitations, API tokens with scoped access, and background deploy jobs so operators and developers share one system.",
    icon: Shield,
  },
  {
    title: "Visibility into every deploy",
    body: "Logs, health checks, and roadmap surfaces for metrics and terminals—so you are not flying blind after a release.",
    icon: Terminal,
  },
];

export function HomePage() {
  return (
    <div>
      <section className="border-b border-slate-200/80 bg-gradient-to-b from-brand-50/50 via-slate-50 to-slate-50/90">
        <div className="mx-auto max-w-6xl px-4 py-16 md:py-24">
          <p className="text-xs font-semibold uppercase tracking-widest text-slate-500">
            DeployWerk V2 (preview)
          </p>
          <h1 className="mt-3 max-w-3xl text-3xl font-semibold tracking-tight text-slate-900 md:text-4xl lg:text-[2.5rem] lg:leading-tight">
            The deploy experience teams expect—on infrastructure you own
          </h1>
          <p className="mt-6 max-w-2xl text-base leading-relaxed text-slate-600 md:text-lg">
            Git-backed workflows, environments, deploy jobs, and domains in a single dashboard, similar in spirit to
            Vercel or Netlify—but you bring the servers and Docker runtime. Rust API, React UI, and CLI; behavior matches
            the open-source tree and README.
          </p>
          <p className="mt-4 max-w-2xl text-sm text-slate-600">
            Preview-quality UI today: full app shell, deployments, settings, and clear placeholders for edge analytics
            and deeper integrations—see the repo roadmap for parity with global edge platforms.
          </p>
          <div className="mt-10 flex flex-wrap gap-3">
            <Link to="/register" className="dw-btn-primary gap-2 px-5 py-3">
              Create account
              <ArrowRight className="h-4 w-4" strokeWidth={1.75} />
            </Link>
            <Link to="/login" className="dw-btn-secondary px-5 py-3">
              Sign in
            </Link>
            <Link
              to="/demo"
              className="dw-btn-secondary border-dashed px-5 py-3 text-slate-700"
            >
              Sample logins
            </Link>
          </div>
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-4 py-16">
        <h2 className="text-xl font-semibold tracking-tight text-slate-900 md:text-2xl">Why teams choose this shape</h2>
        <p className="mt-2 max-w-2xl text-sm leading-relaxed text-slate-600 md:text-base">
          One product surface for who ships what, where it runs, and what broke last—without giving up ownership of
          machines and data. Managed-global-edge features are phased; the navigation already reflects the long-term
          story so you can evaluate honestly against Vercel-class platforms.
        </p>
        <ul className="mt-10 grid gap-5 md:grid-cols-2">
          {features.map(({ title, body, icon: Icon }) => (
            <li key={title} className="dw-card p-6 transition hover:shadow-md">
              <div className="flex h-10 w-10 items-center justify-center rounded-md border border-slate-200 bg-slate-50 text-slate-700">
                <Icon className="h-5 w-5" strokeWidth={1.75} />
              </div>
              <h3 className="mt-4 text-base font-semibold text-slate-900">{title}</h3>
              <p className="mt-2 text-sm leading-relaxed text-slate-600">{body}</p>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
