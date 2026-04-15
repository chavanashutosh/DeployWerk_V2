import type { ReactNode } from "react";
import { Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { Toaster } from "sonner";
import { useAuth } from "./auth";
import { MarketingLayout } from "./components/MarketingLayout";
import { AppShell } from "./components/AppShell";
import * as L from "./lazyPages";
import { AdminGate } from "./pages/admin/AdminGate";

function RouteFallback() {
  return (
    <div className="flex min-h-[40vh] items-center justify-center bg-slate-50 px-4">
      <div className="dw-card flex min-w-[200px] flex-col items-center gap-3 px-8 py-8">
        <div
          className="h-7 w-7 animate-spin rounded-full border-2 border-slate-200 border-t-slate-800"
          aria-hidden
        />
        <p className="text-sm font-medium text-slate-600">Loading…</p>
      </div>
    </div>
  );
}

function Protected({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-slate-50 px-4">
        <div className="dw-card flex min-w-[240px] flex-col items-center gap-4 px-8 py-10">
          <div
            className="h-8 w-8 animate-spin rounded-full border-2 border-slate-200 border-t-slate-800"
            aria-hidden
          />
          <p className="text-sm font-medium text-slate-600">Loading…</p>
        </div>
      </div>
    );
  }
  if (!user) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

const pricingRequiresAuth = import.meta.env.VITE_PRICING_REQUIRES_AUTH === "true";

export default function App() {
  return (
    <Suspense fallback={<RouteFallback />}>
      <Toaster richColors position="top-center" />
      <Routes>
        <Route element={<MarketingLayout />}>
          <Route path="/" element={<L.HomePage />} />
          <Route
            path="/pricing"
            element={
              pricingRequiresAuth ? (
                <Protected>
                  <L.PricingPage />
                </Protected>
              ) : (
                <L.PricingPage />
              )
            }
          />
          <Route path="/login" element={<L.LoginPage />} />
          <Route path="/login/oidc/callback" element={<L.OidcCallbackPage />} />
          <Route path="/register" element={<L.RegisterPage />} />
          <Route path="/demo" element={<L.DemoPage />} />
          <Route path="/invite/:token" element={<L.InvitePage />} />
          <Route path="/legal/terms" element={<L.TermsPage />} />
          <Route path="/legal/privacy" element={<L.PrivacyPage />} />
        </Route>
        <Route
          path="/app"
          element={
            <Protected>
              <AppShell />
            </Protected>
          }
        >
          <Route index element={<L.DashboardPage />} />
          <Route path="orgs/:orgId/settings" element={<L.OrganizationSettingsPage />} />
          <Route path="settings/tokens" element={<L.TokensPage />} />
          <Route path="teams/:teamId/projects" element={<L.ProjectsPage />} />
          <Route
            path="teams/:teamId/projects/:projectId/environments"
            element={<L.EnvironmentsPage />}
          />
          <Route
            path="teams/:teamId/projects/:projectId/environments/:environmentId/applications"
            element={<L.ApplicationsPage />}
          />
          <Route path="teams/:teamId/invite" element={<L.TeamInvitePage />} />
          <Route path="teams/:teamId/servers" element={<L.ServersPage />} />
          <Route path="teams/:teamId/servers/:serverId/docker" element={<L.ServerDockerPage />} />
          <Route path="teams/:teamId/destinations" element={<L.DestinationsPage />} />
          <Route path="teams/:teamId/deployments" element={<L.DeploymentsPage />} />
          <Route path="teams/:teamId/logs" element={<L.LogsPage />} />
          <Route path="teams/:teamId/domains" element={<L.DomainsPage />} />
          <Route path="teams/:teamId/search" element={<L.SearchPage />} />
          <Route path="teams/:teamId/analytics" element={<L.AnalyticsPageApp />} />
          <Route path="teams/:teamId/speed-insights" element={<L.SpeedInsightsPageApp />} />
          <Route path="teams/:teamId/observability" element={<L.ObservabilityPageApp />} />
          <Route path="teams/:teamId/firewall" element={<L.FirewallPageApp />} />
          <Route path="teams/:teamId/cdn" element={<L.CdnPageApp />} />
          <Route path="teams/:teamId/integrations" element={<L.IntegrationsPageApp />} />
          <Route path="teams/:teamId/storage" element={<L.StoragePageApp />} />
          <Route path="teams/:teamId/flags" element={<L.FlagsPageApp />} />
          <Route path="teams/:teamId/agent" element={<L.AgentPageApp />} />
          <Route path="teams/:teamId/ai-gateway" element={<L.AiGatewayPageApp />} />
          <Route path="teams/:teamId/sandboxes" element={<L.SandboxesPageApp />} />
          <Route path="teams/:teamId/usage" element={<L.UsagePageApp />} />
          <Route path="teams/:teamId/support" element={<L.SupportPageApp />} />
          <Route path="teams/:teamId/cli" element={<L.WebCliPage />} />
          <Route path="teams/:teamId/settings" element={<L.SettingsLayout />}>
            <Route index element={<Navigate to="general" replace />} />
            <Route path="general" element={<L.GeneralSettingsPage />} />
            <Route path="team" element={<L.TeamSettingsPage />} />
            <Route path="notifications" element={<L.NotificationsSettingsPage />} />
            <Route path="secrets" element={<L.TeamSecretsSettingsPage />} />
            <Route path="domains" element={<L.SettingsDomainsPage />} />
            <Route path="audit-log" element={<L.TeamAuditSettingsPage />} />
            <Route path="mail-domains" element={<L.MailDomainsSettingsPage />} />
            <Route path="mail" element={<L.MailOverviewSettingsPage />} />
          </Route>
        </Route>
        <Route
          path="/admin"
          element={
            <Protected>
              <AdminGate>
                <L.AdminShell />
              </AdminGate>
            </Protected>
          }
        >
          <Route index element={<L.AdminDashboardPage />} />
          <Route path="users" element={<L.AdminUsersPage />} />
          <Route path="users/:id" element={<L.AdminUserDetailPage />} />
          <Route path="organizations" element={<L.AdminOrganizationsPage />} />
          <Route path="organizations/:id" element={<L.AdminOrganizationDetailPage />} />
          <Route path="teams" element={<L.AdminTeamsPage />} />
          <Route path="teams/:teamId/entitlements" element={<L.AdminTeamEntitlementsPage />} />
          <Route path="teams/:teamId" element={<L.AdminTeamDetailPage />} />
          <Route path="billing" element={<L.AdminBillingPage />} />
          <Route path="pricing" element={<L.AdminPricingPage />} />
          <Route path="audit" element={<L.AdminAuditPage />} />
          <Route path="system" element={<L.AdminSystemPage />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Suspense>
  );
}
