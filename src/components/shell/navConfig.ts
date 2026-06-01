import type { ComponentType } from "react";
import {
  Activity,
  BarChart3,
  Server,
  Book,
  Wrench,
  Bot,
  Network,
  History,
  Globe,
  FolderOpen,
  KeyRound,
  Shield,
  Cpu,
  Brain,
  Settings,
} from "lucide-react";
import { McpIcon } from "@/components/BrandIcons";
import type { AppId } from "@/lib/api";

/**
 * 所有一级视图。相比旧版新增 dashboard / usage / proxy 三个一级视图，
 * usage 与 proxy 由原设置 Tab 提升而来。
 */
export type View =
  | "dashboard"
  | "providers"
  | "usage"
  | "mcp"
  | "prompts"
  | "skills"
  | "skillsDiscovery"
  | "agents"
  | "proxy"
  | "sessions"
  | "universal"
  | "workspace"
  | "openclawEnv"
  | "openclawTools"
  | "openclawAgents"
  | "hermesMemory"
  | "settings";

/** 图标统一接受 size / className，兼容 lucide 与自定义品牌图标 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type IconComponent = ComponentType<any>;

export interface NavContext {
  activeApp: AppId;
  sharedFeatureApp: AppId;
  hasSkillsSupport: boolean;
  hasSessionSupport: boolean;
}

export interface NavItem {
  view: View;
  /** i18n key */
  labelKey: string;
  /** 缺失翻译时的兜底文案（英文） */
  fallback: string;
  icon: IconComponent;
  visible?: (ctx: NavContext) => boolean;
}

export interface NavGroup {
  id: string;
  titleKey?: string;
  titleFallback?: string;
  /** 动态分组标题（如 App 专属分组用 App 名称） */
  dynamicTitle?: (ctx: NavContext) => string | undefined;
  items: NavItem[];
}

const isClaudeFamily = (app: AppId) =>
  app === "claude" || app === "claude-desktop";

const NAV_GROUPS: NavGroup[] = [
  {
    id: "overview",
    titleKey: "sidebar.groupOverview",
    titleFallback: "Overview",
    items: [
      {
        view: "dashboard",
        labelKey: "sidebar.dashboard",
        fallback: "Activity",
        icon: Activity,
      },
      {
        view: "usage",
        labelKey: "sidebar.usage",
        fallback: "Usage",
        icon: BarChart3,
      },
    ],
  },
  {
    id: "core",
    titleKey: "sidebar.groupCore",
    titleFallback: "Core",
    items: [
      {
        view: "providers",
        labelKey: "sidebar.providers",
        fallback: "Providers",
        icon: Server,
      },
      {
        view: "mcp",
        labelKey: "sidebar.mcp",
        fallback: "MCP",
        icon: McpIcon,
        visible: (ctx) => ctx.activeApp !== "openclaw",
      },
      {
        view: "prompts",
        labelKey: "sidebar.prompts",
        fallback: "Prompts",
        icon: Book,
        visible: (ctx) =>
          ctx.activeApp !== "openclaw" && ctx.activeApp !== "hermes",
      },
      {
        view: "skills",
        labelKey: "sidebar.skills",
        fallback: "Skills",
        icon: Wrench,
        visible: (ctx) => ctx.hasSkillsSupport,
      },
      {
        view: "agents",
        labelKey: "sidebar.agents",
        fallback: "Agents",
        icon: Bot,
        visible: (ctx) => isClaudeFamily(ctx.activeApp),
      },
    ],
  },
  {
    id: "advanced",
    titleKey: "sidebar.groupAdvanced",
    titleFallback: "Advanced",
    items: [
      {
        view: "proxy",
        labelKey: "sidebar.proxy",
        fallback: "Proxy",
        icon: Network,
      },
      {
        view: "sessions",
        labelKey: "sidebar.sessions",
        fallback: "Sessions",
        icon: History,
        visible: (ctx) => ctx.hasSessionSupport,
      },
      {
        view: "universal",
        labelKey: "sidebar.universal",
        fallback: "Universal",
        icon: Globe,
      },
    ],
  },
  {
    id: "appSpecific",
    dynamicTitle: (ctx) => {
      if (ctx.activeApp === "openclaw") return "OpenClaw";
      if (ctx.activeApp === "hermes") return "Hermes";
      return undefined;
    },
    items: [
      {
        view: "workspace",
        labelKey: "sidebar.workspace",
        fallback: "Workspace",
        icon: FolderOpen,
        visible: (ctx) => ctx.activeApp === "openclaw",
      },
      {
        view: "openclawEnv",
        labelKey: "sidebar.env",
        fallback: "Environment",
        icon: KeyRound,
        visible: (ctx) => ctx.activeApp === "openclaw",
      },
      {
        view: "openclawTools",
        labelKey: "sidebar.tools",
        fallback: "Tools",
        icon: Shield,
        visible: (ctx) => ctx.activeApp === "openclaw",
      },
      {
        view: "openclawAgents",
        labelKey: "sidebar.agentDefaults",
        fallback: "Agent Defaults",
        icon: Cpu,
        visible: (ctx) => ctx.activeApp === "openclaw",
      },
      {
        view: "hermesMemory",
        labelKey: "sidebar.memory",
        fallback: "Memory",
        icon: Brain,
        visible: (ctx) => ctx.activeApp === "hermes",
      },
    ],
  },
  {
    id: "bottom",
    items: [
      {
        view: "settings",
        labelKey: "sidebar.settings",
        fallback: "Settings",
        icon: Settings,
      },
    ],
  },
];

/** 返回当前上下文下可见的导航分组（已过滤空分组与隐藏项） */
export function getNavGroups(ctx: NavContext): NavGroup[] {
  return NAV_GROUPS.map((group) => ({
    ...group,
    items: group.items.filter((item) => !item.visible || item.visible(ctx)),
  })).filter((group) => group.items.length > 0);
}

/** 所有合法视图（用于 localStorage 校验） */
export const VALID_VIEWS: View[] = [
  "dashboard",
  "providers",
  "usage",
  "mcp",
  "prompts",
  "skills",
  "skillsDiscovery",
  "agents",
  "proxy",
  "sessions",
  "universal",
  "workspace",
  "openclawEnv",
  "openclawTools",
  "openclawAgents",
  "hermesMemory",
  "settings",
];
