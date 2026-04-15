import { useEffect, useState, type FormEvent } from "react";
import { KeyRound, Trash2 } from "lucide-react";
import { apiFetch } from "@/api";
import { toastError, toastSuccess } from "@/toast";
import { InlineError, PageHeader } from "@/components/ui";

type TokenRow = {
  id: string;
  name: string;
  scopes: { read: boolean; write: boolean; deploy: boolean };
  created_at: string;
  expires_at?: string | null;
  allowed_cidrs?: string[] | null;
};

export function TokensPage() {
  const [tokens, setTokens] = useState<TokenRow[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [createdOnce, setCreatedOnce] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [scopes, setScopes] = useState({ read: true, write: false, deploy: false });
  const [expiresInDays, setExpiresInDays] = useState<number | "">("");
  const [allowedCidrsStr, setAllowedCidrsStr] = useState("");
  const [pending, setPending] = useState(false);

  async function load() {
    try {
      const list = await apiFetch<TokenRow[]>("/api/v1/tokens");
      setTokens(list);
      setErr(null);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "Failed to load");
      setTokens(null);
    }
  }

  useEffect(() => {
    void load();
  }, []);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    const scopeList: string[] = [];
    if (scopes.read) scopeList.push("read");
    if (scopes.write) scopeList.push("write");
    if (scopes.deploy) scopeList.push("deploy");
    if (scopeList.length === 0) {
      const msg = "Select at least one scope";
      setErr(msg);
      toastError(msg);
      return;
    }
    setPending(true);
    setErr(null);
    setCreatedOnce(null);
    try {
      const expires_in_days =
        typeof expiresInDays === "number" && Number.isFinite(expiresInDays) && expiresInDays > 0
          ? Math.floor(expiresInDays)
          : undefined;
      const allowed_cidrs = allowedCidrsStr
        .split(/[\n,]+/)
        .map((s) => s.trim())
        .filter(Boolean);
      const res = await apiFetch<{ token: string }>("/api/v1/tokens", {
        method: "POST",
        body: JSON.stringify({
          name: name.trim(),
          scopes: scopeList,
          expires_in_days,
          allowed_cidrs: allowed_cidrs.length ? allowed_cidrs : undefined,
        }),
      });
      setCreatedOnce(res.token);
      setName("");
      setExpiresInDays("");
      setAllowedCidrsStr("");
      toastSuccess("Token created — copy it below");
      await load();
    } catch (err2) {
      const msg = err2 instanceof Error ? err2.message : "Create failed";
      setErr(msg);
      toastError(msg);
    } finally {
      setPending(false);
    }
  }

  async function revoke(id: string) {
    if (!confirm("Revoke this token?")) return;
    try {
      await apiFetch(`/api/v1/tokens/${id}`, { method: "DELETE" });
      toastSuccess("Token revoked");
      await load();
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Revoke failed";
      setErr(msg);
      toastError(msg);
    }
  }

  return (
    <div className="space-y-8">
      <PageHeader
        icon={<KeyRound className="h-6 w-6" strokeWidth={1.75} />}
        title="API tokens"
        description={
          <>
            Personal tokens use the same{" "}
            <code className="rounded bg-slate-100 px-1 font-mono text-xs">Authorization: Bearer</code> header as your
            login JWT. Create tokens only while signed in with your password (not with an API token).
          </>
        }
      />

      <InlineError message={err} />

      <div className="dw-card p-6 sm:p-8">
        <h2 className="text-lg font-semibold text-slate-900">Create token</h2>
        {createdOnce && (
          <div className="mt-4 rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-950">
            <p className="font-semibold">Copy this token now — it will not be shown again.</p>
            <code className="mt-2 block break-all font-mono text-xs">{createdOnce}</code>
          </div>
        )}
        <form className="mt-4 space-y-4" onSubmit={onCreate}>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="tname">
              Name
            </label>
            <input
              id="tname"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="dw-input mt-1 max-w-md"
              required
            />
          </div>
          <fieldset className="flex flex-wrap gap-4 text-sm">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={scopes.read}
                onChange={(e) => setScopes((s) => ({ ...s, read: e.target.checked }))}
              />
              read
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={scopes.write}
                onChange={(e) => setScopes((s) => ({ ...s, write: e.target.checked }))}
              />
              write
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={scopes.deploy}
                onChange={(e) => setScopes((s) => ({ ...s, deploy: e.target.checked }))}
              />
              deploy
            </label>
          </fieldset>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="exp">
              Expiry (days)
            </label>
            <input
              id="exp"
              type="number"
              min={1}
              max={3650}
              value={expiresInDays}
              onChange={(e) => setExpiresInDays(e.target.value ? Number(e.target.value) : "")}
              className="dw-input mt-1 w-40"
              placeholder="optional"
            />
            <p className="mt-1 text-xs text-slate-500">
              Optional. Leave empty for non-expiring tokens.
            </p>
          </div>
          <div>
            <label className="block text-sm font-medium text-slate-700" htmlFor="cidr">
              Allowed CIDRs (optional)
            </label>
            <textarea
              id="cidr"
              value={allowedCidrsStr}
              onChange={(e) => setAllowedCidrsStr(e.target.value)}
              rows={2}
              className="dw-input mt-1 max-w-xl font-mono text-xs"
              placeholder={"203.0.113.0/24\n2001:db8::/32"}
            />
            <p className="mt-1 text-xs text-slate-500">
              When set, this token only works from matching client IPs. The API uses{" "}
              <code className="rounded bg-slate-100 px-1">X-Forwarded-For</code> /{" "}
              <code className="rounded bg-slate-100 px-1">X-Real-IP</code> — configure your proxy accordingly.
            </p>
          </div>
          <button type="submit" disabled={pending} className="dw-btn-primary">
            {pending ? "Creating…" : "Create token"}
          </button>
        </form>
      </div>

      <div className="dw-card p-6 sm:p-8">
        <h2 className="text-lg font-semibold text-slate-900">Your tokens</h2>
        {!tokens || tokens.length === 0 ? (
          <p className="mt-4 text-sm text-slate-600">No tokens yet.</p>
        ) : (
          <ul className="mt-4 divide-y divide-slate-100">
            {tokens.map((t) => (
              <li
                key={t.id}
                className="flex flex-wrap items-center justify-between gap-2 py-4 first:pt-0"
              >
                <div>
                  <p className="font-medium text-slate-900">{t.name}</p>
                  <p className="text-xs text-slate-500">
                    {[t.scopes.read && "read", t.scopes.write && "write", t.scopes.deploy && "deploy"]
                      .filter(Boolean)
                      .join(", ")}{" "}
                    · created {new Date(t.created_at).toLocaleString()}
                    {t.expires_at ? ` · expires ${new Date(t.expires_at).toLocaleString()}` : ""}
                    {t.allowed_cidrs?.length
                      ? ` · IP allowlist: ${t.allowed_cidrs.join(", ")}`
                      : ""}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => void revoke(t.id)}
                  className="inline-flex items-center gap-1 rounded-lg border border-red-200 px-2 py-1 text-sm text-red-700 hover:bg-red-50"
                >
                  <Trash2 className="h-4 w-4" strokeWidth={1.75} />
                  Revoke
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
