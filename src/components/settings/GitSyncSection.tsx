import { useCallback, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import {
  Link2,
  UploadCloud,
  DownloadCloud,
  Loader2,
  Save,
  Check,
  AlertTriangle,
  Info,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { settingsApi } from "@/lib/api";
import type { GitSyncSettings, GitAssetInfo } from "@/types";

/** i18n 标签 key：把资产 id 里的 '.' 换成 '_'（避免被当作嵌套分隔符）。 */
function assetLabelKey(id: string): string {
  return `settings.gitSync.assets.${id.replace(/\./g, "_")}`;
}

// ─── Types ──────────────────────────────────────────────────

type ActionState = "idle" | "testing" | "saving" | "pushing" | "pulling";

interface GitSyncSectionProps {
  config?: GitSyncSettings;
}

// ─── ActionButton ───────────────────────────────────────────

function ActionButton({
  actionState,
  targetState,
  icon: Icon,
  activeLabel,
  idleLabel,
  disabled,
  ...props
}: {
  actionState: ActionState;
  targetState: ActionState;
  icon: LucideIcon;
  activeLabel: ReactNode;
  idleLabel: ReactNode;
} & Omit<React.ComponentPropsWithoutRef<typeof Button>, "children">) {
  const isActive = actionState === targetState;
  return (
    <Button {...props} disabled={actionState !== "idle" || disabled}>
      <span className="inline-flex items-center gap-2">
        {isActive ? (
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
        ) : (
          <Icon className="h-3.5 w-3.5" />
        )}
        {isActive ? activeLabel : idleLabel}
      </span>
    </Button>
  );
}

// ─── Main component ─────────────────────────────────────────

export function GitSyncSection({ config }: GitSyncSectionProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [actionState, setActionState] = useState<ActionState>("idle");
  const [dirty, setDirty] = useState(false);
  const [tokenTouched, setTokenTouched] = useState(false);
  const [justSaved, setJustSaved] = useState(false);
  const justSavedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [showDiverged, setShowDiverged] = useState(false);
  const [catalog, setCatalog] = useState<GitAssetInfo[]>([]);

  // Local form state — token is only persisted on explicit "Save".
  const [form, setForm] = useState(() => ({
    repoUrl: config?.repoUrl ?? "",
    branch: config?.branch ?? "main",
    username: config?.username ?? "",
    token: config?.token ?? "",
    authorName: config?.authorName ?? "",
    authorEmail: config?.authorEmail ?? "",
    deviceName: config?.deviceName ?? "",
    disabledAssets: config?.disabledAssets ?? [],
  }));

  useEffect(() => {
    return () => {
      if (justSavedTimerRef.current) clearTimeout(justSavedTimerRef.current);
    };
  }, []);

  // Load the asset catalog once.
  useEffect(() => {
    let cancelled = false;
    settingsApi
      .gitSyncAssetCatalog()
      .then((items) => {
        if (!cancelled) setCatalog(items);
      })
      .catch(() => {
        /* catalog is best-effort */
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Sync form when backend config loads/updates, unless the user is editing.
  useEffect(() => {
    if (!config || dirty) return;
    setForm({
      repoUrl: config.repoUrl ?? "",
      branch: config.branch ?? "main",
      username: config.username ?? "",
      token: config.token ?? "",
      authorName: config.authorName ?? "",
      authorEmail: config.authorEmail ?? "",
      deviceName: config.deviceName ?? "",
      disabledAssets: config.disabledAssets ?? [],
    });
    setTokenTouched(false);
  }, [config, dirty]);

  const clearJustSaved = useCallback(() => {
    setJustSaved(false);
    if (justSavedTimerRef.current) {
      clearTimeout(justSavedTimerRef.current);
      justSavedTimerRef.current = null;
    }
  }, []);

  const updateField = useCallback(
    (
      field: "repoUrl" | "branch" | "username" | "token" | "deviceName",
      value: string,
    ) => {
      setForm((prev) => ({ ...prev, [field]: value }));
      if (field === "token") setTokenTouched(true);
      setDirty(true);
      clearJustSaved();
    },
    [clearJustSaved],
  );

  const toggleAsset = useCallback(
    (id: string, enabled: boolean) => {
      setForm((prev) => {
        const set = new Set(prev.disabledAssets);
        if (enabled) set.delete(id);
        else set.add(id);
        return { ...prev, disabledAssets: Array.from(set) };
      });
      setDirty(true);
      clearJustSaved();
    },
    [clearJustSaved],
  );

  const buildSettings = useCallback((): GitSyncSettings | null => {
    const repoUrl = form.repoUrl.trim();
    if (!repoUrl) return null;
    return {
      enabled: true,
      repoUrl,
      branch: form.branch.trim() || "main",
      username: form.username.trim(),
      // 未重新触碰 token 时提交空值，后端沿用已保存 token；表单值仅用于 UI 显示
      token: tokenTouched ? form.token : "",
      authorName: form.authorName.trim(),
      authorEmail: form.authorEmail.trim(),
      deviceName: form.deviceName.trim(),
      disabledAssets: form.disabledAssets,
    };
  }, [form, tokenTouched]);

  // ─── Handlers ───────────────────────────────────────────

  const handleTest = useCallback(async () => {
    const settings = buildSettings();
    if (!settings) {
      toast.error(t("settings.gitSync.missingUrl"));
      return;
    }
    setActionState("testing");
    try {
      await settingsApi.gitTestConnection(settings, tokenTouched);
      toast.success(t("settings.gitSync.testSuccess"));
    } catch (error) {
      toast.error(
        t("settings.gitSync.testFailed", {
          error: (error as Error)?.message ?? String(error),
        }),
      );
    } finally {
      setActionState("idle");
    }
  }, [buildSettings, tokenTouched, t]);

  const handleSave = useCallback(async () => {
    const settings = buildSettings();
    if (!settings) {
      toast.error(t("settings.gitSync.missingUrl"));
      return;
    }
    setActionState("saving");
    try {
      await settingsApi.gitSyncSaveSettings(settings, tokenTouched);
      setDirty(false);
      setTokenTouched(false);
      setJustSaved(true);
      if (justSavedTimerRef.current) clearTimeout(justSavedTimerRef.current);
      justSavedTimerRef.current = setTimeout(() => {
        setJustSaved(false);
        justSavedTimerRef.current = null;
      }, 2000);
      await queryClient.invalidateQueries();
      toast.success(t("settings.gitSync.saveSuccess"));
    } catch (error) {
      toast.error(
        t("settings.gitSync.saveFailed", {
          error: (error as Error)?.message ?? String(error),
        }),
      );
    } finally {
      setActionState("idle");
    }
  }, [buildSettings, tokenTouched, queryClient, t]);

  const handlePush = useCallback(async () => {
    if (dirty) {
      toast.error(t("settings.gitSync.unsavedChanges"));
      return;
    }
    setActionState("pushing");
    try {
      const res = await settingsApi.gitSyncPush();
      if (res.diverged) {
        setShowDiverged(true);
      } else if (res.noChanges) {
        toast.info(t("settings.gitSync.noChanges"));
      } else {
        toast.success(t("settings.gitSync.pushSuccess"));
        await queryClient.invalidateQueries();
      }
    } catch (error) {
      toast.error(
        t("settings.gitSync.pushFailed", {
          error: (error as Error)?.message ?? String(error),
        }),
      );
    } finally {
      setActionState("idle");
    }
  }, [dirty, queryClient, t]);

  const handleForcePush = useCallback(async () => {
    setShowDiverged(false);
    setActionState("pushing");
    try {
      await settingsApi.gitSyncForcePush();
      toast.success(t("settings.gitSync.pushSuccess"));
      await queryClient.invalidateQueries();
    } catch (error) {
      toast.error(
        t("settings.gitSync.pushFailed", {
          error: (error as Error)?.message ?? String(error),
        }),
      );
    } finally {
      setActionState("idle");
    }
  }, [queryClient, t]);

  const handlePull = useCallback(async () => {
    setShowDiverged(false);
    if (dirty) {
      toast.error(t("settings.gitSync.unsavedChanges"));
      return;
    }
    setActionState("pulling");
    try {
      const res = await settingsApi.gitSyncPull();
      if (res.empty) {
        toast.info(t("settings.gitSync.noRemoteData"));
      } else {
        toast.success(t("settings.gitSync.pullSuccess"));
        if (res.warning) toast.warning(res.warning);
        await queryClient.invalidateQueries();
      }
    } catch (error) {
      toast.error(
        t("settings.gitSync.pullFailed", {
          error: (error as Error)?.message ?? String(error),
        }),
      );
    } finally {
      setActionState("idle");
    }
  }, [dirty, queryClient, t]);

  // ─── Derived state ──────────────────────────────────────

  const isLoading = actionState !== "idle";
  const hasSavedConfig = Boolean(config?.repoUrl?.trim());
  const lastSyncAt = config?.status?.lastSyncAt;
  const lastSyncDisplay = lastSyncAt
    ? new Date(lastSyncAt).toLocaleString()
    : null;
  const lastError = config?.status?.lastError?.trim();

  // ─── Render ─────────────────────────────────────────────

  return (
    <section className="space-y-4">
      <header className="space-y-2">
        <h3 className="text-base font-semibold text-foreground">
          {t("settings.gitSync.title")}
        </h3>
        <p className="text-sm text-muted-foreground">
          {t("settings.gitSync.description")}
        </p>
      </header>

      <div className="space-y-4 rounded-lg border border-border bg-muted/40 p-6">
        <div className="space-y-3">
          {/* Repo URL */}
          <div className="flex items-center gap-4">
            <label className="w-40 shrink-0 text-xs font-medium text-foreground">
              {t("settings.gitSync.repoUrl")}
            </label>
            <Input
              value={form.repoUrl}
              onChange={(e) => updateField("repoUrl", e.target.value)}
              placeholder="https://github.com/owner/repo.git"
              className="flex-1 text-xs"
              disabled={isLoading}
            />
          </div>

          {/* Branch */}
          <div className="flex items-center gap-4">
            <label className="w-40 shrink-0 text-xs font-medium text-foreground">
              {t("settings.gitSync.branch")}
            </label>
            <Input
              value={form.branch}
              onChange={(e) => updateField("branch", e.target.value)}
              placeholder="main"
              className="flex-1 text-xs"
              disabled={isLoading}
            />
          </div>

          {/* Token (PAT) */}
          <div className="flex items-center gap-4">
            <label className="w-40 shrink-0 text-xs font-medium text-foreground">
              {t("settings.gitSync.token")}
            </label>
            <Input
              type="password"
              value={form.token}
              onChange={(e) => updateField("token", e.target.value)}
              placeholder={t("settings.gitSync.tokenPlaceholder")}
              className="flex-1 text-xs"
              autoComplete="off"
              disabled={isLoading}
            />
          </div>

          {/* Username (optional) */}
          <div className="flex items-center gap-4">
            <label className="w-40 shrink-0 text-xs font-medium text-foreground">
              {t("settings.gitSync.username")}
              <span className="block text-[10px] font-normal text-muted-foreground">
                {t("settings.gitSync.usernameHint")}
              </span>
            </label>
            <Input
              value={form.username}
              onChange={(e) => updateField("username", e.target.value)}
              placeholder="x-access-token"
              className="flex-1 text-xs"
              disabled={isLoading}
            />
          </div>

          {/* Device name (optional) */}
          <div className="flex items-center gap-4">
            <label className="w-40 shrink-0 text-xs font-medium text-foreground">
              {t("settings.gitSync.deviceName")}
              <span className="block text-[10px] font-normal text-muted-foreground">
                {t("settings.gitSync.deviceNameHint")}
              </span>
            </label>
            <Input
              value={form.deviceName}
              onChange={(e) => updateField("deviceName", e.target.value)}
              placeholder={t("settings.gitSync.deviceNamePlaceholder")}
              className="flex-1 text-xs"
              disabled={isLoading}
            />
          </div>

          <div className="flex items-start gap-2 text-xs text-muted-foreground">
            <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            <span>{t("settings.gitSync.privateRepoHint")}</span>
          </div>
        </div>

        {/* Asset selection */}
        {catalog.length > 0 && (
          <div className="space-y-3 border-t border-border pt-4">
            <div>
              <p className="text-xs font-medium text-foreground">
                {t("settings.gitSync.assets.title")}
              </p>
              <p className="text-[10px] text-muted-foreground">
                {t("settings.gitSync.assets.hint")}
              </p>
            </div>
            <div className="grid grid-cols-1 gap-x-6 gap-y-2 sm:grid-cols-2">
              {catalog.map((asset) => {
                const enabled = !form.disabledAssets.includes(asset.id);
                return (
                  <label
                    key={asset.id}
                    className="flex items-start gap-2.5 text-xs"
                    title={asset.homePath}
                  >
                    <Checkbox
                      checked={enabled}
                      onCheckedChange={(c) => toggleAsset(asset.id, c === true)}
                      disabled={isLoading}
                      className="mt-0.5"
                    />
                    <span className="leading-tight">
                      <span className="font-medium text-foreground">
                        {t(assetLabelKey(asset.id))}
                      </span>
                      <span className="block text-[10px] text-muted-foreground">
                        {asset.homePath}
                      </span>
                    </span>
                  </label>
                );
              })}
            </div>
            <p className="text-[10px] text-muted-foreground">
              {t("settings.gitSync.assets.coreNote")}
            </p>
          </div>
        )}

        {/* Last sync time */}
        {lastSyncDisplay && (
          <p className="text-xs text-muted-foreground">
            {t("settings.gitSync.lastSync", { time: lastSyncDisplay })}
          </p>
        )}
        {lastError && config?.status?.lastErrorSource === "manual" && (
          <div className="rounded-lg border border-red-300/70 bg-red-50/80 px-3 py-2 text-xs text-red-900 dark:border-red-500/50 dark:bg-red-950/30 dark:text-red-200">
            <p className="font-medium">
              {t("settings.gitSync.lastErrorTitle")}
            </p>
            <p className="mt-1 whitespace-pre-wrap break-all">{lastError}</p>
          </div>
        )}

        {/* Config buttons + save status */}
        <div className="flex flex-wrap items-center gap-3 pt-2">
          <ActionButton
            type="button"
            variant="outline"
            size="sm"
            onClick={handleTest}
            actionState={actionState}
            targetState="testing"
            icon={Link2}
            activeLabel={t("settings.gitSync.testing")}
            idleLabel={t("settings.gitSync.test")}
          />
          <ActionButton
            type="button"
            variant="outline"
            size="sm"
            onClick={handleSave}
            actionState={actionState}
            targetState="saving"
            icon={Save}
            activeLabel={t("settings.gitSync.saving")}
            idleLabel={t("settings.gitSync.save")}
          />
          {dirty && (
            <span className="inline-flex items-center gap-1.5 text-xs text-amber-500 dark:text-amber-400">
              <span className="h-1.5 w-1.5 rounded-full bg-amber-500 dark:bg-amber-400" />
              {t("settings.gitSync.unsaved")}
            </span>
          )}
          {!dirty && justSaved && (
            <span className="inline-flex items-center gap-1.5 text-xs text-emerald-600 dark:text-emerald-400">
              <Check className="h-3 w-3" />
              {t("settings.gitSync.saved")}
            </span>
          )}
        </div>

        {/* Sync buttons */}
        <div className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
          <ActionButton
            type="button"
            size="sm"
            onClick={handlePush}
            disabled={!hasSavedConfig}
            actionState={actionState}
            targetState="pushing"
            icon={UploadCloud}
            activeLabel={t("settings.gitSync.pushing")}
            idleLabel={t("settings.gitSync.push")}
          />
          <ActionButton
            type="button"
            variant="secondary"
            size="sm"
            onClick={handlePull}
            disabled={!hasSavedConfig}
            actionState={actionState}
            targetState="pulling"
            icon={DownloadCloud}
            activeLabel={t("settings.gitSync.pulling")}
            idleLabel={t("settings.gitSync.pull")}
          />
        </div>
        {!hasSavedConfig && (
          <p className="text-xs text-muted-foreground">
            {t("settings.gitSync.saveBeforeSync")}
          </p>
        )}
      </div>

      {/* ─── Diverged resolution dialog ──────────────────── */}
      <Dialog
        open={showDiverged}
        onOpenChange={(open) => {
          if (!open) setShowDiverged(false);
        }}
      >
        <DialogContent className="max-w-sm" zIndex="alert">
          <DialogHeader className="space-y-3 border-b-0 bg-transparent pb-0">
            <DialogTitle className="flex items-center gap-2 text-lg font-semibold">
              <AlertTriangle className="h-5 w-5 text-destructive" />
              {t("settings.gitSync.diverged.title")}
            </DialogTitle>
            <DialogDescription asChild>
              <div className="space-y-3 text-sm leading-relaxed">
                <p>{t("settings.gitSync.diverged.content")}</p>
                <ul className="list-disc space-y-1 pl-5 text-muted-foreground">
                  <li>{t("settings.gitSync.diverged.pullOption")}</li>
                  <li>{t("settings.gitSync.diverged.forceOption")}</li>
                </ul>
              </div>
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="flex gap-2 border-t-0 bg-transparent pt-2 sm:justify-end">
            <Button variant="outline" onClick={() => setShowDiverged(false)}>
              {t("common.cancel")}
            </Button>
            <Button variant="secondary" onClick={handlePull}>
              {t("settings.gitSync.diverged.pull")}
            </Button>
            <Button variant="destructive" onClick={handleForcePush}>
              {t("settings.gitSync.diverged.force")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
