type Props = {
  label?: string;
  className?: string;
};

/** Non-blocking loading placeholder for main content areas. */
export function LoadingBlock({ label = "Loading…", className = "" }: Props) {
  return (
    <div
      className={`dw-card flex items-center gap-3 px-6 py-10 ${className}`}
      aria-busy
      aria-live="polite"
    >
      <div
        className="h-5 w-5 shrink-0 animate-spin rounded-full border-2 border-slate-200 border-t-slate-700"
        aria-hidden
      />
      <p className="text-sm font-medium text-slate-600">{label}</p>
    </div>
  );
}
