import { lazy } from "react";

/** Route-level code splits: each import() becomes an async chunk (placeholders share one chunk). */
export const HomePage = lazy(() => import("./pages/HomePage").then((m) => ({ default: m.HomePage })));
export const PricingPage = lazy(() => import("./pages/PricingPage").then((m) => ({ default: m.PricingPage })));
export const LoginPage = lazy(() => import("./pages/LoginPage").then((m) => ({ default: m.LoginPage })));
export const OidcCallbackPage = lazy(() =>
  import("./pages/OidcCallbackPage").then((m) => ({ default: m.OidcCallbackPage })),
);
export const RegisterPage = lazy(() => import("./pages/RegisterPage").then((m) => ({ default: m.RegisterPage })));
export const DemoPage = lazy(() => import("./pages/DemoPage").then((m) => ({ default: m.DemoPage })));
export const DashboardPage = lazy(() => import("./pages/DashboardPage").then((m) => ({ default: m.DashboardPage })));
export const TermsPage = lazy(() => import("./pages/TermsPage").then((m) => ({ default: m.TermsPage })));
export const PrivacyPage = lazy(() => import("./pages/PrivacyPage").then((m) => ({ default: m.PrivacyPage })));
export const ProjectsPage = lazy(() => import("./pages/ProjectsPage").then((m) => ({ default: m.ProjectsPage })));
export const EnvironmentsPage = lazy(() =>
  import("./pages/EnvironmentsPage").then((m) => ({ default: m.EnvironmentsPage })),
);
export const TokensPage = lazy(() => import("./pages/TokensPage").then((m) => ({ default: m.TokensPage })));
export const SsoPlaybookPage = lazy(() =>
  import("./pages/app/SsoPlaybookPage").then((m) => ({ default: m.SsoPlaybookPage })),
);
export const InvitePage = lazy(() => import("./pages/InvitePage").then((m) => ({ default: m.InvitePage })));
export const TeamInvitePage = lazy(() => import("./pages/TeamInvitePage").then((m) => ({ default: m.TeamInvitePage })));
export const ServersPage = lazy(() => import("./pages/ServersPage").then((m) => ({ default: m.ServersPage })));
export const ServerDockerPage = lazy(() =>
  import("./pages/ServerDockerPage").then((m) => ({ default: m.ServerDockerPage })),
);
export const DestinationsPage = lazy(() =>
  import("./pages/DestinationsPage").then((m) => ({ default: m.DestinationsPage })),
);
export const ApplicationsPage = lazy(() =>
  import("./pages/ApplicationsPage").then((m) => ({ default: m.ApplicationsPage })),
);
export const DeploymentsPage = lazy(() =>
  import("./pages/app/DeploymentsPage").then((m) => ({ default: m.DeploymentsPage })),
);
export const LogsPage = lazy(() => import("./pages/app/LogsPage").then((m) => ({ default: m.LogsPage })));
export const DomainsPage = lazy(() => import("./pages/app/DomainsPage").then((m) => ({ default: m.DomainsPage })));
export const SearchPage = lazy(() => import("./pages/app/SearchPage").then((m) => ({ default: m.SearchPage })));

export const SettingsLayout = lazy(() =>
  import("./pages/app/SettingsLayout").then((m) => ({ default: m.SettingsLayout })),
);
export const GeneralSettingsPage = lazy(() =>
  import("./pages/app/settings/GeneralSettingsPage").then((m) => ({ default: m.GeneralSettingsPage })),
);
export const TeamSettingsPage = lazy(() =>
  import("./pages/app/settings/TeamSettingsPage").then((m) => ({ default: m.TeamSettingsPage })),
);
export const NotificationsSettingsPage = lazy(() =>
  import("./pages/app/settings/NotificationsSettingsPage").then((m) => ({
    default: m.NotificationsSettingsPage,
  })),
);
export const SettingsDomainsPage = lazy(() =>
  import("./pages/app/settings/SettingsDomainsPage").then((m) => ({ default: m.SettingsDomainsPage })),
);
export const TeamSecretsSettingsPage = lazy(() =>
  import("./pages/app/settings/TeamSecretsSettingsPage").then((m) => ({ default: m.TeamSecretsSettingsPage })),
);
export const OrganizationSettingsPage = lazy(() =>
  import("./pages/app/settings/OrganizationSettingsPage").then((m) => ({ default: m.OrganizationSettingsPage })),
);
export const TeamAuditSettingsPage = lazy(() =>
  import("./pages/app/settings/TeamAuditSettingsPage").then((m) => ({ default: m.TeamAuditSettingsPage })),
);
export const MailDomainsSettingsPage = lazy(() =>
  import("./pages/app/settings/MailDomainsSettingsPage").then((m) => ({ default: m.MailDomainsSettingsPage })),
);
export const MailOverviewSettingsPage = lazy(() =>
  import("./pages/app/settings/MailOverviewSettingsPage").then((m) => ({ default: m.MailOverviewSettingsPage })),
);

const ph = () => import("./pages/app/placeholders");
export const AnalyticsPageApp = lazy(() => ph().then((m) => ({ default: m.AnalyticsPageApp })));
export const SpeedInsightsPageApp = lazy(() => ph().then((m) => ({ default: m.SpeedInsightsPageApp })));
export const ObservabilityPageApp = lazy(() => ph().then((m) => ({ default: m.ObservabilityPageApp })));
export const FirewallPageApp = lazy(() => ph().then((m) => ({ default: m.FirewallPageApp })));
export const CdnPageApp = lazy(() => ph().then((m) => ({ default: m.CdnPageApp })));
export const IntegrationsPageApp = lazy(() => ph().then((m) => ({ default: m.IntegrationsPageApp })));
export const StoragePageApp = lazy(() => ph().then((m) => ({ default: m.StoragePageApp })));
export const FlagsPageApp = lazy(() => ph().then((m) => ({ default: m.FlagsPageApp })));
export const AgentPageApp = lazy(() => ph().then((m) => ({ default: m.AgentPageApp })));
export const AiGatewayPageApp = lazy(() => ph().then((m) => ({ default: m.AiGatewayPageApp })));
export const SandboxesPageApp = lazy(() => ph().then((m) => ({ default: m.SandboxesPageApp })));
export const UsagePageApp = lazy(() => ph().then((m) => ({ default: m.UsagePageApp })));
export const SupportPageApp = lazy(() => ph().then((m) => ({ default: m.SupportPageApp })));
export const WebCliPage = lazy(() =>
  import("./pages/app/WebCliPage").then((m) => ({ default: m.WebCliPage })),
);

export const AdminShell = lazy(() => import("./pages/admin/AdminShell").then((m) => ({ default: m.AdminShell })));
export const AdminDashboardPage = lazy(() =>
  import("./pages/admin/AdminDashboardPage").then((m) => ({ default: m.AdminDashboardPage })),
);
export const AdminUsersPage = lazy(() => import("./pages/admin/AdminUsersPage").then((m) => ({ default: m.AdminUsersPage })));
export const AdminUserDetailPage = lazy(() =>
  import("./pages/admin/AdminUserDetailPage").then((m) => ({ default: m.AdminUserDetailPage })),
);
export const AdminOrganizationsPage = lazy(() =>
  import("./pages/admin/AdminOrganizationsPage").then((m) => ({ default: m.AdminOrganizationsPage })),
);
export const AdminOrganizationDetailPage = lazy(() =>
  import("./pages/admin/AdminOrganizationDetailPage").then((m) => ({ default: m.AdminOrganizationDetailPage })),
);
export const AdminTeamsPage = lazy(() => import("./pages/admin/AdminTeamsPage").then((m) => ({ default: m.AdminTeamsPage })));
export const AdminTeamDetailPage = lazy(() =>
  import("./pages/admin/AdminTeamDetailPage").then((m) => ({ default: m.AdminTeamDetailPage })),
);
export const AdminBillingPage = lazy(() =>
  import("./pages/admin/AdminBillingPage").then((m) => ({ default: m.AdminBillingPage })),
);
export const AdminTeamEntitlementsPage = lazy(() =>
  import("./pages/admin/AdminTeamEntitlementsPage").then((m) => ({ default: m.AdminTeamEntitlementsPage })),
);
export const AdminAuditPage = lazy(() => import("./pages/admin/AdminAuditPage").then((m) => ({ default: m.AdminAuditPage })));
export const AdminSystemPage = lazy(() =>
  import("./pages/admin/AdminSystemPage").then((m) => ({ default: m.AdminSystemPage })),
);
export const AdminPricingPage = lazy(() =>
  import("./pages/admin/AdminPricingPage").then((m) => ({ default: m.AdminPricingPage })),
);
