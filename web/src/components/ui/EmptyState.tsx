import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

type Props = {
  icon: LucideIcon;
  title: string;
  children: ReactNode;
  /** Primary CTA (e.g. Link or button) */
  action?: ReactNode;
};

export function EmptyState({ icon: Icon, title, children, action }: Props) {
  return (
    <div className="dw-card flex flex-col items-center justify-center px-6 py-12 text-center">
      <span className="flex h-12 w-12 items-center justify-center rounded-full bg-slate-100 text-slate-600">
        <Icon className="h-6 w-6" strokeWidth={1.5} />
      </span>
      <h2 className="mt-4 text-base font-semibold text-slate-900">{title}</h2>
      <p className="mt-2 max-w-md text-sm leading-relaxed text-slate-600">{children}</p>
      {action ? <div className="mt-6">{action}</div> : null}
    </div>
  );
}
