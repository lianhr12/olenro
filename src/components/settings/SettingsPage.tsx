import { useCallback, useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  Loader2,
  Save,
  ArrowLeft,
  SlidersHorizontal,
  Palette,
  FolderSearch,
  Database,
  Cloud,
  KeyRound,
  FlaskConical,
  ScrollText,
  Info,
  Coins,
  type LucideIcon,
} from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { settingsApi } from "@/lib/api";
import { LanguageSettings } from "@/components/settings/LanguageSettings";
import { ThemeSettings } from "@/components/settings/ThemeSettings";
import { WindowSettings } from "@/components/settings/WindowSettings";
import { AppVisibilitySettings } from "@/components/settings/AppVisibilitySettings";
import { SkillStorageLocationSettings } from "@/components/settings/SkillStorageLocationSettings";
import { SkillSyncMethodSettings } from "@/components/settings/SkillSyncMethodSettings";
import { TerminalSettings } from "@/components/settings/TerminalSettings";
import { DirectorySettings } from "@/components/settings/DirectorySettings";
import { ImportExportSection } from "@/components/settings/ImportExportSection";
import { BackupListSection } from "@/components/settings/BackupListSection";
import { GitSyncSection } from "@/components/settings/GitSyncSection";
import { AboutSection } from "@/components/settings/AboutSection";
import { ModelTestConfigPanel } from "@/components/usage/ModelTestConfigPanel";
import { PricingConfigPanel } from "@/components/usage/PricingConfigPanel";
import { LogConfigPanel } from "@/components/settings/LogConfigPanel";
import { AuthCenterPanel } from "@/components/settings/AuthCenterPanel";
import { useInstalledSkills } from "@/hooks/useSkills";
import { useSettings } from "@/hooks/useSettings";
import { useImportExport } from "@/hooks/useImportExport";
import { useTranslation } from "react-i18next";
import type { SettingsFormState } from "@/hooks/useSettings";

interface SettingsDialogProps {
  open: boolean;
  onImportSuccess?: () => void | Promise<void>;
  defaultTab?: string;
}

interface SettingsCategory {
  id: string;
  titleKey: string;
  descKey: string;
  icon: LucideIcon;
  iconClass: string;
}

const CATEGORIES: SettingsCategory[] = [
  {
    id: "general",
    titleKey: "settingsGrid.general",
    descKey: "settingsGrid.generalDesc",
    icon: SlidersHorizontal,
    iconClass: "text-blue-500 bg-blue-500/10",
  },
  {
    id: "appearance",
    titleKey: "settingsGrid.appearance",
    descKey: "settingsGrid.appearanceDesc",
    icon: Palette,
    iconClass: "text-violet-500 bg-violet-500/10",
  },
  {
    id: "directories",
    titleKey: "settingsGrid.directories",
    descKey: "settingsGrid.directoriesDesc",
    icon: FolderSearch,
    iconClass: "text-amber-500 bg-amber-500/10",
  },
  {
    id: "data",
    titleKey: "settingsGrid.data",
    descKey: "settingsGrid.dataDesc",
    icon: Database,
    iconClass: "text-cyan-500 bg-cyan-500/10",
  },
  {
    id: "cloudSync",
    titleKey: "settingsGrid.cloudSync",
    descKey: "settingsGrid.cloudSyncDesc",
    icon: Cloud,
    iconClass: "text-sky-500 bg-sky-500/10",
  },
  {
    id: "auth",
    titleKey: "settingsGrid.auth",
    descKey: "settingsGrid.authDesc",
    icon: KeyRound,
    iconClass: "text-emerald-500 bg-emerald-500/10",
  },
  {
    id: "modelTest",
    titleKey: "settingsGrid.modelTest",
    descKey: "settingsGrid.modelTestDesc",
    icon: FlaskConical,
    iconClass: "text-pink-500 bg-pink-500/10",
  },
  {
    id: "pricing",
    titleKey: "settingsGrid.costPricing",
    descKey: "settingsGrid.costPricingDesc",
    icon: Coins,
    iconClass: "text-yellow-500 bg-yellow-500/10",
  },
  {
    id: "logs",
    titleKey: "settingsGrid.logs",
    descKey: "settingsGrid.logsDesc",
    icon: ScrollText,
    iconClass: "text-orange-500 bg-orange-500/10",
  },
  {
    id: "about",
    titleKey: "settingsGrid.about",
    descKey: "settingsGrid.aboutDesc",
    icon: Info,
    iconClass: "text-gray-500 bg-gray-500/10",
  },
];

const CATEGORY_IDS = CATEGORIES.map((c) => c.id);

export function SettingsPage({
  open,
  onImportSuccess,
  defaultTab = "general",
}: SettingsDialogProps) {
  const { t } = useTranslation();
  const {
    settings,
    isLoading,
    isSaving,
    isPortable,
    appConfigDir,
    resolvedDirs,
    updateSettings,
    updateDirectory,
    updateAppConfigDir,
    browseDirectory,
    browseAppConfigDir,
    resetDirectory,
    resetAppConfigDir,
    saveSettings,
    autoSaveSettings,
    requiresRestart,
    acknowledgeRestart,
  } = useSettings();

  const {
    selectedFile,
    status: importStatus,
    errorMessage,
    backupId,
    isImporting,
    selectImportFile,
    importConfig,
    exportConfig,
    clearSelection,
    resetStatus,
  } = useImportExport({ onImportSuccess });

  const { data: installedSkills } = useInstalledSkills();

  // null = 显示九宫格入口；否则显示对应分类详情
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const [showRestartPrompt, setShowRestartPrompt] = useState(false);

  useEffect(() => {
    if (open) {
      // 仅在被显式深链到某个分类（如 about）时直接进入详情，否则展示入口网格
      setActiveCategory(
        defaultTab &&
          defaultTab !== "general" &&
          CATEGORY_IDS.includes(defaultTab)
          ? defaultTab
          : null,
      );
      resetStatus();
    }
  }, [open, resetStatus, defaultTab]);

  useEffect(() => {
    if (requiresRestart) {
      setShowRestartPrompt(true);
    }
  }, [requiresRestart]);

  const closeAfterSave = useCallback(() => {
    acknowledgeRestart();
    clearSelection();
    resetStatus();
    setActiveCategory(null);
  }, [acknowledgeRestart, clearSelection, resetStatus]);

  const handleSave = useCallback(async () => {
    try {
      const result = await saveSettings(undefined, { silent: false });
      if (!result) return;
      if (result.requiresRestart) {
        setShowRestartPrompt(true);
        return;
      }
      closeAfterSave();
    } catch (error) {
      console.error("[SettingsPage] Failed to save settings", error);
    }
  }, [closeAfterSave, saveSettings]);

  const handleRestartLater = useCallback(() => {
    setShowRestartPrompt(false);
    closeAfterSave();
  }, [closeAfterSave]);

  const handleRestartNow = useCallback(async () => {
    setShowRestartPrompt(false);
    if (import.meta.env.DEV) {
      toast.success(t("settings.devModeRestartHint"), { closeButton: true });
      closeAfterSave();
      return;
    }
    try {
      await settingsApi.restart();
    } catch (error) {
      console.error("[SettingsPage] Failed to restart app", error);
      toast.error(t("settings.restartFailed"));
    } finally {
      closeAfterSave();
    }
  }, [closeAfterSave, t]);

  const handleAutoSave = useCallback(
    async (updates: Partial<SettingsFormState>) => {
      if (!settings) return;
      updateSettings(updates);
      try {
        await autoSaveSettings(updates);
      } catch (error) {
        console.error("[SettingsPage] Failed to autosave settings", error);
        toast.error(
          t("settings.saveFailedGeneric", {
            defaultValue: "保存失败，请重试",
          }),
        );
      }
    },
    [autoSaveSettings, settings, t, updateSettings],
  );

  const isBusy = useMemo(() => isLoading && !settings, [isLoading, settings]);

  const activeCategoryMeta = CATEGORIES.find((c) => c.id === activeCategory);

  const renderDetail = () => {
    switch (activeCategory) {
      case "general":
        return settings ? (
          <div className="space-y-6">
            <LanguageSettings
              value={settings.language}
              onChange={(lang) => handleAutoSave({ language: lang })}
            />
            <AppVisibilitySettings
              settings={settings}
              onChange={handleAutoSave}
            />
            <SkillStorageLocationSettings
              value={settings.skillStorageLocation ?? "cc_switch"}
              installedCount={installedSkills?.length ?? 0}
              onMigrated={(location) =>
                updateSettings({ skillStorageLocation: location })
              }
            />
            <SkillSyncMethodSettings
              value={settings.skillSyncMethod ?? "auto"}
              onChange={(method) => handleAutoSave({ skillSyncMethod: method })}
            />
            <WindowSettings settings={settings} onChange={handleAutoSave} />
            <TerminalSettings
              value={settings.preferredTerminal}
              onChange={(terminal) =>
                handleAutoSave({ preferredTerminal: terminal })
              }
            />
          </div>
        ) : null;
      case "appearance":
        return <ThemeSettings />;
      case "directories":
        return settings ? (
          <DirectorySettings
            appConfigDir={appConfigDir}
            resolvedDirs={resolvedDirs}
            onAppConfigChange={updateAppConfigDir}
            onBrowseAppConfig={browseAppConfigDir}
            onResetAppConfig={resetAppConfigDir}
            claudeDir={settings.claudeConfigDir}
            codexDir={settings.codexConfigDir}
            geminiDir={settings.geminiConfigDir}
            opencodeDir={settings.opencodeConfigDir}
            openclawDir={settings.openclawConfigDir}
            hermesDir={settings.hermesConfigDir}
            onDirectoryChange={updateDirectory}
            onBrowseDirectory={browseDirectory}
            onResetDirectory={resetDirectory}
          />
        ) : null;
      case "data":
        return settings ? (
          <div className="space-y-6">
            <ImportExportSection
              status={importStatus}
              selectedFile={selectedFile}
              errorMessage={errorMessage}
              backupId={backupId}
              isImporting={isImporting}
              onSelectFile={selectImportFile}
              onImport={importConfig}
              onExport={exportConfig}
              onClear={clearSelection}
            />
            <BackupListSection
              backupIntervalHours={settings.backupIntervalHours}
              backupRetainCount={settings.backupRetainCount}
              onSettingsChange={(updates) => handleAutoSave(updates)}
            />
          </div>
        ) : null;
      case "cloudSync":
        return <GitSyncSection config={settings?.gitSync} />;
      case "auth":
        return <AuthCenterPanel />;
      case "modelTest":
        return <ModelTestConfigPanel />;
      case "pricing":
        return <PricingConfigPanel />;
      case "logs":
        return <LogConfigPanel />;
      case "about":
        return <AboutSection isPortable={isPortable} />;
      default:
        return null;
    }
  };

  return (
    <div className="flex h-full flex-col overflow-hidden px-6 pb-4 pt-2">
      {isBusy ? (
        <div className="flex flex-1 items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      ) : activeCategory ? (
        // ===== 分类详情 =====
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="mb-4 flex items-center gap-3">
            <Button
              variant="outline"
              size="icon"
              onClick={() => setActiveCategory(null)}
              className="rounded-lg"
              aria-label={t("common.back")}
            >
              <ArrowLeft className="h-4 w-4" />
            </Button>
            <h2 className="text-lg font-semibold">
              {activeCategoryMeta && t(activeCategoryMeta.titleKey)}
            </h2>
          </div>
          <div className="flex-1 overflow-y-auto overflow-x-hidden pr-2">
            <motion.div
              key={activeCategory}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.25 }}
              className="pb-4"
            >
              {renderDetail()}
            </motion.div>
          </div>
          {activeCategory === "directories" && settings && (
            <div
              className="flex-shrink-0 border-t border-border-default pt-4"
              style={{ backgroundColor: "hsl(var(--background))" }}
            >
              <div className="flex items-center justify-end gap-3">
                <Button onClick={handleSave} disabled={isSaving}>
                  {isSaving ? (
                    <span className="inline-flex items-center gap-2">
                      <Loader2 className="h-4 w-4 animate-spin" />
                      {t("settings.saving")}
                    </span>
                  ) : (
                    <>
                      <Save className="mr-2 h-4 w-4" />
                      {t("common.save")}
                    </>
                  )}
                </Button>
              </div>
            </div>
          )}
        </div>
      ) : (
        // ===== 九宫格入口 =====
        <div className="flex-1 overflow-y-auto pr-2">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {CATEGORIES.map((cat) => {
              const Icon = cat.icon;
              return (
                <button
                  key={cat.id}
                  type="button"
                  onClick={() => setActiveCategory(cat.id)}
                  className="glass-card group flex flex-col items-start gap-3 rounded-2xl p-5 text-left transition-all hover:-translate-y-0.5 hover:shadow-lg"
                >
                  <div
                    className={cn(
                      "flex h-11 w-11 items-center justify-center rounded-xl",
                      cat.iconClass,
                    )}
                  >
                    <Icon className="h-5 w-5" />
                  </div>
                  <div>
                    <div className="text-base font-semibold">
                      {t(cat.titleKey)}
                    </div>
                    <p className="mt-1 text-sm text-muted-foreground">
                      {t(cat.descKey)}
                    </p>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}

      <Dialog
        open={showRestartPrompt}
        onOpenChange={(o) => !o && handleRestartLater()}
      >
        <DialogContent zIndex="alert" className="max-w-md glass border-border">
          <DialogHeader>
            <DialogTitle>{t("settings.restartRequired")}</DialogTitle>
          </DialogHeader>
          <div className="px-6">
            <p className="text-sm text-muted-foreground">
              {t("settings.restartRequiredMessage")}
            </p>
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={handleRestartLater}
              className="hover:bg-muted/50"
            >
              {t("settings.restartLater")}
            </Button>
            <Button
              onClick={handleRestartNow}
              className="bg-primary hover:bg-primary/90"
            >
              {t("settings.restartNow")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
