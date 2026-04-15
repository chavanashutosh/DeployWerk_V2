import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router-dom";

/** User-friendly message when admin API rejects the session. */
export function formatAdminListError(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  if (msg.includes("403") || /\bforbidden\b/i.test(msg)) {
    return "Access denied (403). Sign out and sign in again if your super admin role changed.";
  }
  return msg;
}

type AdminSearchFieldProps = {
  id: string;
  label: string;
  placeholder: string;
  value: string;
  onChange: (v: string) => void;
  icon?: LucideIcon;
};

export function AdminSearchField({
  id,
  label,
  placeholder,
  value,
  onChange,
  icon: Icon,
}: AdminSearchFieldProps) {
  return (
    <div className="max-w-md">
      <label htmlFor={id} className="block text-xs font-medium uppercase tracking-wide text-slate-500">
        {label}
      </label>
      <div className="relative mt-1">
        {Icon ? (
          <Icon
            className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400"
            strokeWidth={1.75}
            aria-hidden
          />
        ) : null}
        <input
          id={id}
          type="search"
          autoComplete="off"
          placeholder={placeholder}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className={`w-full rounded-lg border border-slate-300 bg-white py-2 text-sm text-slate-900 shadow-sm placeholder:text-slate-400 focus:border-slate-500 focus:outline-none focus:ring-1 focus:ring-slate-400 ${
            Icon ? "pl-9 pr-3" : "px-3"
          }`}
        />
      </div>
    </div>
  );
}

export function AdminTableWrap({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={`dw-card overflow-auto rounded-xl ${className ?? ""}`}>{children}</div>
  );
}

export function AdminThead({ children }: { children: ReactNode }) {
  return (
    <thead className="sticky top-0 z-10 border-b border-slate-200 bg-slate-50 text-left text-slate-600 shadow-sm">
      {children}
    </thead>
  );
}

export function AdminLoadingRow({ colSpan }: { colSpan: number }) {
  return (
    <tr>
      <td colSpan={colSpan} className="px-4 py-10 text-center text-sm text-slate-500">
        Loading…
      </td>
    </tr>
  );
}

export function AdminEmptyRow({ colSpan, message }: { colSpan: number; message: string }) {
  return (
    <tr>
      <td colSpan={colSpan} className="px-4 py-10 text-center text-sm text-slate-500">
        {message}
      </td>
    </tr>
  );
}

type AdminViewLinkProps = {
  to: string;
  label: string;
  icon: LucideIcon;
};

/** Icon + text link for table “open detail” actions. */
export function AdminViewLink({ to, label, icon: Icon }: AdminViewLinkProps) {
  return (
    <Link
      to={to}
      className="inline-flex items-center gap-1.5 font-medium text-violet-700 hover:text-violet-900 hover:underline"
    >
      <Icon className="h-4 w-4 shrink-0" strokeWidth={1.75} aria-hidden />
      {label}
    </Link>
  );
}

type AdminIconLinkProps = {
  to: string;
  title: string;
  icon: LucideIcon;
};

/** Icon-only detail link with tooltip. */
export function AdminIconLink({ to, title, icon: Icon }: AdminIconLinkProps) {
  return (
    <Link
      to={to}
      title={title}
      aria-label={title}
      className="inline-flex rounded-md p-1.5 text-slate-500 hover:bg-violet-50 hover:text-violet-700"
    >
      <Icon className="h-4 w-4" strokeWidth={1.75} />
    </Link>
  );
}
