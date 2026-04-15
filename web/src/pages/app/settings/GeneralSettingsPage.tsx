import { FormEvent, useEffect, useState } from "react";
import { Fingerprint, KeyRound, Palette, Save, User } from "lucide-react";
import { useAuth } from "@/auth";
import { patchMe } from "@/api";

export function GeneralSettingsPage() {
  const { user, refresh } = useAuth();
  const [name, setName] = useState("");
  const [theme, setTheme] = useState("system");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    setName(user?.name ?? "");
    const s = user?.settings;
    if (s && typeof s === "object" && !Array.isArray(s) && "theme" in s) {
      const t = (s as { theme?: string }).theme;
      if (typeof t === "string") setTheme(t);
    }
  }, [user]);

  async function saveProfile(e: FormEvent) {
    e.preventDefault();
    setErr(null);
    setMsg(null);
    try {
      const settings = {
        ...(typeof user?.settings === "object" && user?.settings !== null && !Array.isArray(user.settings)
          ? user.settings
          : {}),
        theme,
      };
      await patchMe({ name: name || undefined, settings });
      await refresh();
      setMsg("Saved profile.");
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : "Save failed");
    }
  }

  async function changePassword(e: FormEvent) {
    e.preventDefault();
    setErr(null);
    setMsg(null);
    try {
      await patchMe({
        current_password: currentPassword,
        new_password: newPassword,
      });
      setCurrentPassword("");
      setNewPassword("");
      await refresh();
      setMsg("Password updated.");
    } catch (ex) {
      setErr(ex instanceof Error ? ex.message : "Password change failed");
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-start gap-3">
        <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-brand-100 text-brand-800">
          <User className="h-5 w-5" strokeWidth={1.75} />
        </span>
        <div>
          <h2 className="text-lg font-semibold text-slate-900">General</h2>
          <p className="mt-1 text-sm text-slate-600">Account details and preferences for your user.</p>
        </div>
      </div>
      {msg && <p className="text-sm text-emerald-700">{msg}</p>}
      {err && <p className="text-sm text-red-600">{err}</p>}
      <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h3 className="text-sm font-semibold text-slate-900">Signed-in user</h3>
        <p className="mt-2 font-mono text-sm text-slate-700">{user?.email}</p>
        {user?.id && (
          <p className="mt-2 flex items-center gap-2 text-xs text-slate-500">
            <Fingerprint className="h-3.5 w-3.5 shrink-0" strokeWidth={1.75} />
            <span className="font-mono">{user.id}</span>
          </p>
        )}
        <form onSubmit={saveProfile} className="mt-4 space-y-4">
          <label className="block text-sm">
            <span className="text-slate-600">Display name</span>
            <input
              className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <label className="block text-sm">
            <span className="flex items-center gap-1.5 text-slate-600">
              <Palette className="h-3.5 w-3.5 text-slate-400" strokeWidth={1.75} />
              Theme preference
            </span>
            <select
              className="mt-1 w-full max-w-md rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={theme}
              onChange={(e) => setTheme(e.target.value)}
            >
              <option value="system">System</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
            </select>
          </label>
          <button
            type="submit"
            className="inline-flex items-center gap-2 rounded-lg bg-brand-600 px-4 py-2 text-sm font-medium text-white"
          >
            <Save className="h-4 w-4" strokeWidth={1.75} />
            Save profile
          </button>
        </form>
      </div>
      <div className="rounded-xl border border-slate-200 bg-white p-6 shadow-sm">
        <h3 className="flex items-center gap-2 text-sm font-semibold text-slate-900">
          <KeyRound className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
          Change password
        </h3>
        <form onSubmit={changePassword} className="mt-4 grid max-w-md gap-3">
          <label className="text-sm">
            <span className="text-slate-600">Current password</span>
            <input
              type="password"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={currentPassword}
              onChange={(e) => setCurrentPassword(e.target.value)}
              autoComplete="current-password"
            />
          </label>
          <label className="text-sm">
            <span className="text-slate-600">New password (min 8)</span>
            <input
              type="password"
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2 text-sm"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              autoComplete="new-password"
            />
          </label>
          <button
            type="submit"
            className="inline-flex items-center gap-2 rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium"
          >
            <KeyRound className="h-4 w-4 text-slate-500" strokeWidth={1.75} />
            Update password
          </button>
        </form>
      </div>
    </div>
  );
}
