import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  apiFetch,
  getToken,
  putCurrentOrganization,
  putCurrentTeam,
  setToken,
  type User,
} from "./api";

type AuthState = {
  user: User | null;
  loading: boolean;
  refresh: () => Promise<void>;
  login: (token: string) => Promise<void>;
  logout: () => void;
  setCurrentTeam: (teamId: string) => Promise<void>;
  setCurrentOrganization: (organizationId: string) => Promise<void>;
};

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    const t = getToken();
    if (!t) {
      setUser(null);
      setLoading(false);
      return;
    }
    try {
      const me = await apiFetch<User>("/api/v1/me");
      setUser(me);
    } catch {
      setToken(null);
      setUser(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const login = useCallback(async (token: string) => {
    setToken(token);
    setLoading(true);
    await refresh();
  }, [refresh]);

  const logout = useCallback(() => {
    setToken(null);
    setUser(null);
  }, []);

  const setCurrentTeam = useCallback(
    async (teamId: string) => {
      await putCurrentTeam(teamId);
      await refresh();
    },
    [refresh],
  );

  const setCurrentOrganization = useCallback(
    async (organizationId: string) => {
      await putCurrentOrganization(organizationId);
      await refresh();
    },
    [refresh],
  );

  const value = useMemo(
    () => ({
      user,
      loading,
      refresh,
      login,
      logout,
      setCurrentTeam,
      setCurrentOrganization,
    }),
    [user, loading, refresh, login, logout, setCurrentTeam, setCurrentOrganization],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth outside AuthProvider");
  return ctx;
}
