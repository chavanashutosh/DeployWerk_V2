import type { ReactNode } from "react";
import { NavLink } from "react-router-dom";
import type { LucideIcon } from "lucide-react";
import {
  Activity,
  Bell,
  Bot,
  Box,
  Cloud,
  Flag,
  FolderKanban,
  Gauge,
  HardDrive,
  LayoutDashboard,
  Layers,
  LifeBuoy,
  LineChart,
  Link2,
  Mail,
  MailPlus,
  Puzzle,
  Rocket,
  Server,
  Settings,
  Shield,
  Sparkles,
  Terminal,
  Wallet,
} from "lucide-react";

const navCls = ({ isActive }: { isActive: boolean }) =>
  `flex items-center gap-2 rounded-md border-l-2 py-2 pl-2 pr-3 text-sm font-medium transition-colors ${
    isActive
      ? "border-white bg-slate-800 text-white"
      : "border-transparent text-slate-400 hover:bg-slate-800/60 hover:text-slate-200"
  }`;

function NavItem({
  to,
  icon: Icon,
  label,
  end,
}: {
  to: string;
  icon: LucideIcon;
  label: string;
  end?: boolean;
}) {
  return (
    <NavLink to={to} end={end} className={navCls}>
      <Icon className="h-4 w-4 shrink-0" strokeWidth={1.75} />
      {label}
    </NavLink>
  );
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mt-5 first:mt-0">
      <p className="mb-2 px-3 text-[10px] font-semibold uppercase tracking-widest text-slate-500">
        {title}
      </p>
      <div className="flex flex-col gap-0.5">{children}</div>
    </div>
  );
}

type Props = {
  teamId: string;
  canInvite: boolean;
  onNavigate?: () => void;
};

export function SidebarNav({ teamId, canInvite, onNavigate }: Props) {
  const base = `/app/teams/${teamId}`;
  const wrapNavigate = onNavigate ?? (() => {});

  return (
    <nav className="flex flex-col py-4" onClick={() => wrapNavigate()}>
      <div className="px-3">
        <NavItem to="/app" icon={LayoutDashboard} label="Overview" end />
      </div>
      <Group title="Build">
        <NavItem to={`${base}/projects`} icon={FolderKanban} label="Projects" />
        <NavItem to={`${base}/deployments`} icon={Rocket} label="Deployments" />
        <NavItem to={`${base}/logs`} icon={Layers} label="Logs" />
      </Group>
      <Group title="Infrastructure">
        <NavItem to={`${base}/servers`} icon={Server} label="Servers" />
        <NavItem to={`${base}/destinations`} icon={Box} label="Destinations" />
      </Group>
      <Group title="Observe">
        <NavItem to={`${base}/analytics`} icon={LineChart} label="Analytics" />
        <NavItem to={`${base}/speed-insights`} icon={Gauge} label="Speed Insights" />
        <NavItem to={`${base}/observability`} icon={Activity} label="Observability" />
      </Group>
      <Group title="Edge">
        <NavItem to={`${base}/firewall`} icon={Shield} label="Firewall" />
        <NavItem to={`${base}/cdn`} icon={Cloud} label="CDN" />
        <NavItem to={`${base}/domains`} icon={Link2} label="Domains" />
      </Group>
      <Group title="Platform">
        <NavItem to={`${base}/integrations`} icon={Puzzle} label="Integrations" />
        <NavItem to={`${base}/storage`} icon={HardDrive} label="Storage" />
        <NavItem to={`${base}/flags`} icon={Flag} label="Flags" />
        <NavItem to={`${base}/agent`} icon={Bot} label="Agent" />
        <NavItem to={`${base}/ai-gateway`} icon={Sparkles} label="AI Gateway" />
        <NavItem to={`${base}/sandboxes`} icon={Box} label="Sandboxes" />
      </Group>
      <Group title="Email &amp; alerts">
        <NavItem to={`${base}/settings/mail`} icon={Mail} label="Email & mail" />
        <NavItem to={`${base}/settings/mail-domains`} icon={MailPlus} label="Mail domains" />
        <NavItem to={`${base}/settings/notifications`} icon={Bell} label="Notifications" />
      </Group>
      <Group title="Account">
        <NavItem to={`${base}/cli`} icon={Terminal} label="Web CLI" />
        <NavItem to={`${base}/usage`} icon={Wallet} label="Usage" />
        <NavItem to={`${base}/support`} icon={LifeBuoy} label="Support" />
        <NavLink to={`${base}/settings`} end={false} className={navCls}>
          <Settings className="h-4 w-4 shrink-0" strokeWidth={1.75} />
          Settings
        </NavLink>
      </Group>
      <div className="mt-4 border-t border-slate-800 px-3 pt-4">
        <NavLink to="/app/settings/tokens" className={navCls}>
          <Layers className="h-4 w-4 shrink-0" strokeWidth={1.75} />
          API tokens
        </NavLink>
        {canInvite && (
          <NavItem to={`${base}/invite`} icon={MailPlus} label="Invite members" />
        )}
      </div>
      <div className="mt-4 px-3">
        <NavLink
          to="/"
          className="text-sm text-slate-500 hover:text-slate-200"
          onClick={() => wrapNavigate()}
        >
          ← Marketing site
        </NavLink>
      </div>
    </nav>
  );
}
