import { FormEvent, useEffect, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { apiFetch } from "@/api";
import { PageHeader } from "@/components/ui";
import { Terminal } from "lucide-react";
import { toastError } from "@/toast";

type InvokeResponse = {
  exit_code: number;
  stdout: string;
};

export function WebCliPage() {
  const { teamId = "" } = useParams();
  const [history, setHistory] = useState<string>("");
  const [line, setLine] = useState("");
  const [pending, setPending] = useState(false);
  const bottomRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [history]);

  async function runCommand(e: FormEvent) {
    e.preventDefault();
    const cmd = line.trim();
    if (!cmd || !teamId) return;
    setPending(true);
    setHistory((h) => `${h}$ ${cmd}\n`);
    setLine("");
    try {
      const res = await apiFetch<InvokeResponse>(`/api/v1/teams/${teamId}/cli/invoke`, {
        method: "POST",
        body: JSON.stringify({ command_line: cmd }),
      });
      const out = res.stdout || "";
      const suffix = out.endsWith("\n") || out.length === 0 ? "" : "\n";
      setHistory((h) => `${h}${out}${suffix}[exit ${res.exit_code}]\n`);
      if (res.exit_code !== 0) {
        toastError(`Exit ${res.exit_code}`);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Request failed";
      setHistory((h) => `${h}error: ${msg}\n`);
      toastError(msg);
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        icon={<Terminal className="h-6 w-6" strokeWidth={1.75} />}
        title="Web CLI"
        description={
          <>
            Allowlisted commands run on the server with your current session. Type <code className="rounded bg-slate-100 px-1 text-xs">help</code>{" "}
            for the list. For full automation use the{" "}
            <code className="rounded bg-slate-100 px-1 text-xs">deploywerk</code> binary and API tokens.
          </>
        }
      />

      <div className="dw-card overflow-hidden p-0">
        <div className="max-h-[min(480px,55vh)] overflow-y-auto bg-slate-950 px-4 py-3 font-mono text-sm text-slate-100">
          {history.length === 0 ? (
            <p className="text-slate-500">Output appears here. Try: help</p>
          ) : (
            <pre className="whitespace-pre-wrap break-words">{history}</pre>
          )}
          <div ref={bottomRef} />
        </div>
        <form onSubmit={runCommand} className="flex gap-2 border-t border-slate-800 bg-slate-900 px-3 py-2">
          <span className="shrink-0 py-2 font-mono text-xs text-slate-500">$</span>
          <input
            className="dw-input flex-1 border-slate-700 bg-slate-950 font-mono text-sm text-slate-100 placeholder:text-slate-600"
            value={line}
            onChange={(e) => setLine(e.target.value)}
            placeholder={pending ? "Running…" : "projects list"}
            disabled={pending || !teamId}
            autoComplete="off"
            spellCheck={false}
            aria-label="Command"
          />
          <button type="submit" disabled={pending || !teamId} className="dw-btn-primary shrink-0">
            Run
          </button>
        </form>
      </div>
    </div>
  );
}
