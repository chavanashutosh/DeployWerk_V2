import { Link } from "react-router-dom";

export function PublicFooter() {
  return (
    <footer className="border-t border-slate-200 bg-white">
      <div className="mx-auto flex max-w-6xl flex-col gap-4 px-4 py-8 text-sm text-slate-600 md:flex-row md:items-center md:justify-between">
        <p>DeployWerk — self-hosted control plane for your servers.</p>
        <div className="flex gap-4">
          <Link to="/legal/terms" className="hover:text-slate-900">
            Terms
          </Link>
          <Link to="/legal/privacy" className="hover:text-slate-900">
            Privacy
          </Link>
        </div>
      </div>
    </footer>
  );
}
