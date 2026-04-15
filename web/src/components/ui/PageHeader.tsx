import type { ReactNode } from "react";

type Props = {
  title: string;
  description?: ReactNode;
  /** e.g. primary actions */
  actions?: ReactNode;
  /** Optional icon in a rounded square */
  icon?: ReactNode;
  className?: string;
};

/**
 * Consistent page title + description + optional actions for app shell content.
 */
export function PageHeader({ title, description, actions, icon, className = "" }: Props) {
  return (
    <div className={`flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between ${className}`}>
      <div className="flex min-w-0 items-start gap-3">
        {icon ? (
          <span className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-slate-200 bg-slate-50 text-slate-700">
            {icon}
          </span>
        ) : null}
        <div className="min-w-0">
          <h1 className="text-xl font-semibold tracking-tight text-slate-900 sm:text-2xl">{title}</h1>
          {description ? (
            <div className="mt-1.5 text-sm leading-relaxed text-slate-600">{description}</div>
          ) : null}
        </div>
      </div>
      {actions ? <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div> : null}
    </div>
  );
}
