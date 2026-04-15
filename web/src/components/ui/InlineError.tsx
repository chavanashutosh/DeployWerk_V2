type Props = {
  message: string | null | undefined;
  className?: string;
};

/** Consistent destructive / error banner for async failures. */
export function InlineError({ message, className = "" }: Props) {
  if (!message) return null;
  return (
    <div
      role="alert"
      className={`rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-900 ${className}`}
    >
      {message}
    </div>
  );
}
