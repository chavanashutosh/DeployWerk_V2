import type { Organization, Team } from "@/api";

export type AppShellOutletContext = {
  teams: Team[];
  organizations: Organization[];
};
