import { useCallback } from "react";
import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ProxyTabContent } from "@/components/settings/ProxyTabContent";
import { useSettings, type SettingsFormState } from "@/hooks/useSettings";

/**
 * 代理 / 故障转移一级视图。包裹原设置中的 ProxyTabContent，
 * 自带 settings 读取与自动保存逻辑。
 */
export function ProxyPage() {
  const { t } = useTranslation();
  const { settings, updateSettings, autoSaveSettings } = useSettings();

  const handleAutoSave = useCallback(
    async (updates: Partial<SettingsFormState>) => {
      if (!settings) return;
      updateSettings(updates);
      try {
        await autoSaveSettings(updates);
      } catch (error) {
        console.error("[ProxyPage] Failed to autosave settings", error);
        toast.error(
          t("settings.saveFailedGeneric", {
            defaultValue: "保存失败，请重试",
          }),
        );
      }
    },
    [settings, updateSettings, autoSaveSettings, t],
  );

  if (!settings) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-6 pb-8 pt-2">
      <div className="space-y-6">
        <ProxyTabContent settings={settings} onAutoSave={handleAutoSave} />
      </div>
    </div>
  );
}
