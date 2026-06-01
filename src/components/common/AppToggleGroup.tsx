import React from "react";
import { useTranslation } from "react-i18next";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { AppId } from "@/lib/api/types";
import { APP_IDS, APP_ICON_MAP } from "@/config/appConfig";

interface AppToggleGroupProps {
  apps: Partial<Record<AppId, boolean>>;
  onToggle: (app: AppId, enabled: boolean) => void;
  appIds?: AppId[];
  /** 提供时渲染一个「全部」按钮，智能切换：未全开则全开，已全开则全关 */
  onToggleAll?: (enabled: boolean) => void;
}

export const AppToggleGroup: React.FC<AppToggleGroupProps> = ({
  apps,
  onToggle,
  appIds = APP_IDS,
  onToggleAll,
}) => {
  const { t } = useTranslation();
  const allEnabled = appIds.every((app) => apps[app]);
  return (
    <div className="flex items-center gap-1.5 flex-shrink-0">
      {onToggleAll && (
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              type="button"
              onClick={() => onToggleAll(!allEnabled)}
              className={`h-6 px-1.5 rounded text-xs font-medium flex items-center justify-center transition-all border shrink-0 ${
                allEnabled
                  ? "border-border-default text-muted-foreground/60 hover:bg-muted"
                  : "border-primary/30 text-primary hover:bg-primary/10"
              }`}
            >
              {t("common.all")}
            </button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            <p>{allEnabled ? t("common.disableAll") : t("common.enableAll")}</p>
          </TooltipContent>
        </Tooltip>
      )}
      {appIds.map((app) => {
        const { label, icon, activeClass } = APP_ICON_MAP[app];
        const enabled = apps[app];
        return (
          <Tooltip key={app}>
            <TooltipTrigger asChild>
              <button
                type="button"
                onClick={() => onToggle(app, !enabled)}
                className={`w-6 h-6 rounded flex items-center justify-center transition-all shrink-0 ${
                  enabled ? activeClass : "opacity-35 hover:opacity-70"
                }`}
              >
                {icon}
              </button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p>
                {label}
                {enabled ? " ✓" : ""}
              </p>
            </TooltipContent>
          </Tooltip>
        );
      })}
    </div>
  );
};
