import { Link } from "react-router-dom";

export function PublicFooter() {
  return (
    <footer className="border-t border-slate-200 bg-white">
      <div className="mx-auto flex max-w-6xl flex-col gap-4 px-4 py-8 text-sm text-slate-500 md:flex-row md:items-center md:justify-between">
        <p>
          DeployWerk — deploy platform for teams; run it yourself or follow the roadmap for managed options.
        </p>
        <div className="flex gap-6">
          <Link to="/legal/terms" className="dw-link text-sm">
            Terms
          </Link>
          <Link to="/legal/privacy" className="dw-link text-sm">
            Privacy
          </Link>
        </div>
      </div>
    </footer>
  );
}
