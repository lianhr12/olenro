import { useUpdate } from "@/contexts/UpdateContext";
import { useTranslation } from "react-i18next";
import { ArrowUpCircle } from "lucide-react";

interface UpdateBannerProps {
  className?: string;
  onClick?: () => void;
}

/**
 * 侧边栏底部的“有新版本”提示条，仅在检测到更新时显示。
 * 点击后跳转至 设置 → 关于 页面。
 */
export function UpdateBanner({ className = "", onClick }: UpdateBannerProps) {
  const { hasUpdate, updateInfo } = useUpdate();
  const { t } = useTranslation();

  if (!hasUpdate || !updateInfo) {
    return null;
  }

  const version = updateInfo.availableVersion ?? "";
  const label = t("settings.updateAvailableBar", { version });

  return (
    <button
      type="button"
      onClick={onClick}
      title={label}
      aria-label={label}
      className={`flex w-full items-center gap-2 rounded-lg border border-primary/30 bg-primary/10 px-3 py-2 text-sm font-medium text-primary transition-colors hover:bg-primary/20 ${className}`}
    >
      <ArrowUpCircle className="h-4 w-4 shrink-0" />
      <span className="truncate">{label}</span>
    </button>
  );
}
