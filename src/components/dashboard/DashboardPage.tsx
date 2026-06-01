import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Server,
  BarChart3,
  Network,
  Activity,
  ChevronDown,
  CheckCircle2,
  XCircle,
} from "lucide-react";
import type { AppId } from "@/lib/api";
import type { Provider } from "@/types";
import type { UsageRangeSelection } from "@/types/usage";
import type { View } from "@/components/shell/navConfig";
import { cn } from "@/lib/utils";
import { ProviderIcon } from "@/components/ProviderIcon";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import { useProvidersQuery } from "@/lib/query";
import { useUsageSummary, useProviderStats } from "@/lib/query/usage";
import { useProxyStatus } from "@/hooks/useProxyStatus";
import { useProviderActions } from "@/hooks/useProviderActions";
import {
  fmtUsd,
  formatTokensShort,
  getResolvedLang,
} from "@/components/usage/format";

interface DashboardPageProps {
  appId: AppId;
  onNavigate: (view: View) => void;
}

/** 仪表盘用量仅对 claude/codex/gemini 可靠，其余 App 聚合为 all */
function usageAppType(appId: AppId): string {
  if (appId === "claude" || appId === "claude-desktop") return "claude";
  if (appId === "codex") return "codex";
  if (appId === "gemini") return "gemini";
  return "all";
}

function Card({
  title,
  icon: Icon,
  action,
  className,
  children,
}: {
  title: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  icon: React.ComponentType<any>;
  action?: React.ReactNode;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div className={cn("glass-card flex flex-col rounded-2xl p-5", className)}>
      <div className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
          <Icon size={16} className="text-muted-foreground" />
          {title}
        </div>
        {action}
      </div>
      {children}
    </div>
  );
}

export function DashboardPage({ appId, onNavigate }: DashboardPageProps) {
  const { t, i18n } = useTranslation();
  const lang = getResolvedLang(i18n);
  const appType = usageAppType(appId);

  const [rangePreset, setRangePreset] = useState<"today" | "30d">("today");
  const range: UsageRangeSelection = useMemo(
    () => ({ preset: rangePreset }),
    [rangePreset],
  );

  const { isRunning, takeoverStatus, status } = useProxyStatus();
  const isTakeover =
    takeoverStatus?.[appId as "claude" | "codex" | "gemini"] || false;

  const { data: providersData } = useProvidersQuery(appId, {
    isProxyRunning: isRunning,
  });
  const providers = useMemo(
    () => providersData?.providers ?? {},
    [providersData],
  );
  const currentProviderId = providersData?.currentProviderId ?? "";

  const activeProviderId = useMemo(() => {
    const target = status?.active_targets?.find((x) => x.app_type === appId);
    return target?.provider_id ?? currentProviderId;
  }, [status?.active_targets, appId, currentProviderId]);
  const activeProvider: Provider | undefined = providers[activeProviderId];

  const { switchProvider } = useProviderActions(
    appId,
    isRunning,
    isRunning && isTakeover,
  );

  const { data: summary } = useUsageSummary(range, appType);
  const { data: providerStats } = useProviderStats(range, appType);

  const sortedProviders = useMemo(
    () =>
      Object.values(providers).sort(
        (a, b) => (a.sortIndex ?? 0) - (b.sortIndex ?? 0),
      ),
    [providers],
  );

  const healthRows = useMemo(
    () =>
      [...(providerStats ?? [])]
        .sort((a, b) => b.requestCount - a.requestCount)
        .slice(0, 5),
    [providerStats],
  );

  const failoverCount = status?.failover_count ?? 0;

  return (
    <div className="flex-1 overflow-y-auto px-6 pb-8 pt-2">
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        {/* 当前激活服务商 */}
        <Card title={t("dashboard.activeProvider")} icon={Server}>
          <div className="flex items-center gap-3">
            {activeProvider ? (
              <>
                <ProviderIcon
                  icon={activeProvider.icon}
                  name={activeProvider.name}
                  size={36}
                />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-xl font-bold">
                    {activeProvider.name}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {t(`apps.${appId}`, { defaultValue: appId })}
                  </div>
                </div>
              </>
            ) : (
              <div className="flex-1 text-base text-muted-foreground">
                {t("dashboard.noActiveProvider")}
              </div>
            )}
            {sortedProviders.length > 0 && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <button
                    type="button"
                    className="inline-flex items-center gap-1 rounded-lg border border-border bg-card px-3 py-1.5 text-sm font-medium transition-colors hover:bg-accent"
                  >
                    {t("dashboard.quickSwitch")}
                    <ChevronDown className="h-4 w-4" />
                  </button>
                </DropdownMenuTrigger>
                <DropdownMenuContent
                  align="end"
                  className="max-h-72 w-56 overflow-y-auto"
                >
                  {sortedProviders.map((p) => (
                    <DropdownMenuItem
                      key={p.id}
                      onClick={() => void switchProvider(p)}
                      className="gap-2"
                    >
                      <ProviderIcon icon={p.icon} name={p.name} size={16} />
                      <span className="flex-1 truncate">{p.name}</span>
                      {p.id === activeProviderId && (
                        <CheckCircle2 className="h-4 w-4 text-primary" />
                      )}
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            )}
          </div>
        </Card>

        {/* 用量与花费 */}
        <Card
          title={t("dashboard.usageTitle")}
          icon={BarChart3}
          action={
            <div className="flex items-center gap-0.5 rounded-lg bg-muted p-0.5 text-xs">
              {(["today", "30d"] as const).map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => setRangePreset(p)}
                  className={cn(
                    "rounded-md px-2.5 py-1 font-medium transition-colors",
                    rangePreset === p
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {p === "today"
                    ? t("dashboard.usageToday")
                    : t("dashboard.usageMonth")}
                </button>
              ))}
            </div>
          }
        >
          <div className="grid grid-cols-3 gap-3">
            <div>
              <div className="text-2xl font-bold">
                {fmtUsd(summary?.totalCost, 2, "$0.00")}
              </div>
              <div className="text-xs text-muted-foreground">
                {t("dashboard.cost")}
              </div>
            </div>
            <div>
              <div className="text-2xl font-bold">
                {(summary?.totalRequests ?? 0).toLocaleString()}
              </div>
              <div className="text-xs text-muted-foreground">
                {t("dashboard.requests")}
              </div>
            </div>
            <div>
              <div className="text-2xl font-bold">
                {formatTokensShort(summary?.realTotalTokens ?? 0, lang)}
              </div>
              <div className="text-xs text-muted-foreground">
                {t("dashboard.tokens")}
              </div>
            </div>
          </div>
        </Card>

        {/* 代理 / 故障转移 */}
        <Card
          title={t("dashboard.proxyStatus")}
          icon={Network}
          action={
            <button
              type="button"
              onClick={() => onNavigate("proxy")}
              className="text-xs font-medium text-primary hover:underline"
            >
              {t("common.view")}
            </button>
          }
        >
          <div className="flex items-center gap-2">
            <span
              className={cn(
                "inline-block h-2.5 w-2.5 rounded-full",
                isRunning ? "bg-emerald-500" : "bg-muted-foreground/40",
              )}
            />
            <span className="text-xl font-bold">
              {isRunning
                ? t("dashboard.proxyRunning")
                : t("dashboard.proxyStopped")}
            </span>
          </div>
          <div className="mt-3 flex gap-6 text-sm">
            <div>
              <span className="text-muted-foreground">
                {t("dashboard.takeover")}:{" "}
              </span>
              <span className="font-medium">
                {isTakeover ? t("common.enabled") : "—"}
              </span>
            </div>
            <div>
              <span className="text-muted-foreground">
                {t("dashboard.failoverQueue")}:{" "}
              </span>
              <span className="font-medium">{failoverCount}</span>
            </div>
          </div>
        </Card>

        {/* 服务商健康 / 延迟 */}
        <Card
          title={t("dashboard.health")}
          icon={Activity}
          action={
            <button
              type="button"
              onClick={() => onNavigate("usage")}
              className="text-xs font-medium text-primary hover:underline"
            >
              {t("common.view")}
            </button>
          }
        >
          {healthRows.length === 0 ? (
            <div className="py-2 text-sm text-muted-foreground">
              {t("dashboard.noData")}
            </div>
          ) : (
            <div className="space-y-2">
              {healthRows.map((row) => (
                <div
                  key={row.providerId}
                  className="flex items-center gap-2 text-sm"
                >
                  {row.successRate >= 0.95 ? (
                    <CheckCircle2 className="h-4 w-4 shrink-0 text-emerald-500" />
                  ) : (
                    <XCircle className="h-4 w-4 shrink-0 text-amber-500" />
                  )}
                  <span className="min-w-0 flex-1 truncate">
                    {row.providerName}
                  </span>
                  <span className="shrink-0 tabular-nums text-muted-foreground">
                    {Math.round(row.avgLatencyMs)} ms
                  </span>
                </div>
              ))}
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}
