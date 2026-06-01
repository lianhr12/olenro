import { useTranslation } from "react-i18next";
import { ChevronsUpDown, Check } from "lucide-react";
import type { AppId } from "@/lib/api";
import type { VisibleApps } from "@/types";
import { cn } from "@/lib/utils";
import { ProviderIcon } from "@/components/ProviderIcon";
import {
  ALL_APPS,
  APP_DISPLAY_NAME,
  APP_ICON_NAME,
} from "@/components/AppSwitcher";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import { getNavGroups, type NavContext, type View } from "./navConfig";
import { UpdateBanner } from "@/components/UpdateBanner";

const APP_STORAGE_KEY = "olenro-last-app";

interface SidebarProps {
  activeApp: AppId;
  visibleApps: VisibleApps;
  onSwitchApp: (app: AppId) => void;
  currentView: View;
  onNavigate: (view: View) => void;
  navContext: NavContext;
  /** drag bar 高度，用于顶部留白对齐窗口拖拽区 */
  topOffset: number;
  /** 点击“有新版本”提示条时触发，跳转至 设置 → 关于 */
  onShowUpdate?: () => void;
}

export function Sidebar({
  activeApp,
  visibleApps,
  onSwitchApp,
  currentView,
  onNavigate,
  navContext,
  topOffset,
  onShowUpdate,
}: SidebarProps) {
  const { t } = useTranslation();
  const groups = getNavGroups(navContext);

  const appsToShow = ALL_APPS.filter((app) => visibleApps[app] ?? true);

  const handleSwitchApp = (app: AppId) => {
    if (app === activeApp) return;
    localStorage.setItem(APP_STORAGE_KEY, app);
    onSwitchApp(app);
  };

  // settings 项单独固定在底部
  const bottomGroup = groups.find((g) => g.id === "bottom");
  const mainGroups = groups.filter((g) => g.id !== "bottom");

  const renderItem = (
    view: View,
    label: string,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    Icon: React.ComponentType<any>,
  ) => {
    const isActive =
      currentView === view ||
      (view === "skills" && currentView === "skillsDiscovery");
    return (
      <button
        key={view}
        type="button"
        onClick={() => onNavigate(view)}
        className={cn(
          "group flex w-full items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
          isActive
            ? "bg-primary/10 text-primary"
            : "text-muted-foreground hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5",
        )}
      >
        <Icon
          size={18}
          className={cn(
            "shrink-0",
            isActive ? "text-primary" : "text-muted-foreground",
          )}
        />
        <span className="truncate">{label}</span>
      </button>
    );
  };

  return (
    <aside
      className="flex h-full w-[220px] shrink-0 flex-col border-r border-border bg-muted/30"
      style={{ paddingTop: topOffset }}
    >
      {/* App 选择器 */}
      <div className="px-3 py-3">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button
              type="button"
              className="flex w-full items-center gap-2.5 rounded-xl border border-border bg-card px-3 py-2 text-left shadow-sm transition-colors hover:bg-accent"
              title={t("sidebar.selectApp", { defaultValue: "Select app" })}
            >
              <ProviderIcon
                icon={APP_ICON_NAME[activeApp]}
                name={APP_DISPLAY_NAME[activeApp]}
                size={22}
              />
              <span className="min-w-0 flex-1 truncate text-sm font-semibold">
                {APP_DISPLAY_NAME[activeApp]}
              </span>
              <ChevronsUpDown className="h-4 w-4 shrink-0 text-muted-foreground" />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="w-[196px]">
            {appsToShow.map((app) => (
              <DropdownMenuItem
                key={app}
                onClick={() => handleSwitchApp(app)}
                className="gap-2.5"
              >
                <ProviderIcon
                  icon={APP_ICON_NAME[app]}
                  name={APP_DISPLAY_NAME[app]}
                  size={18}
                />
                <span className="flex-1 truncate">{APP_DISPLAY_NAME[app]}</span>
                {app === activeApp && (
                  <Check className="h-4 w-4 text-primary" />
                )}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* 分组导航 */}
      <nav className="flex-1 space-y-4 overflow-y-auto px-3 pb-3">
        {mainGroups.map((group) => {
          const title =
            group.dynamicTitle?.(navContext) ??
            (group.titleKey
              ? t(group.titleKey, { defaultValue: group.titleFallback })
              : undefined);
          return (
            <div key={group.id} className="space-y-1">
              {title && (
                <div className="px-3 pb-1 pt-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground/70">
                  {title}
                </div>
              )}
              {group.items.map((item) =>
                renderItem(
                  item.view,
                  t(item.labelKey, { defaultValue: item.fallback }),
                  item.icon,
                ),
              )}
            </div>
          );
        })}
      </nav>

      {/* 底部：新版本提示 + 设置 */}
      {bottomGroup && (
        <div className="space-y-2 border-t border-border px-3 py-3">
          <UpdateBanner onClick={onShowUpdate} />
          {bottomGroup.items.map((item) =>
            renderItem(
              item.view,
              t(item.labelKey, { defaultValue: item.fallback }),
              item.icon,
            ),
          )}
        </div>
      )}
    </aside>
  );
}
