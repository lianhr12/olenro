import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Minus, Maximize2, Minimize2, X } from "lucide-react";
import { Button } from "@/components/ui/button";

interface AppShellProps {
  /** drag bar 高度（macOS 默认 28；Linux 自定义窗口控件时 32；否则 0） */
  dragBarHeight: number;
  useAppWindowControls: boolean;
  isWindowMaximized: boolean;
  onMinimize: () => void;
  onToggleMaximize: () => void;
  onClose: () => void;
  /** 左侧栏（Sidebar），自行处理 topOffset */
  sidebar: ReactNode;
  /** 顶部标题栏（TopBar） */
  topBar: ReactNode;
  /** 横幅（环境冲突 / OpenClaw 健康提示） */
  banner?: ReactNode;
  children: ReactNode;
}

/**
 * 应用外壳：左侧 Sidebar + 右侧（TopBar + 内容区），顶部覆盖窗口拖拽区。
 * 仅负责结构与窗口控件，业务状态由 App.tsx 注入。
 */
export function AppShell({
  dragBarHeight,
  useAppWindowControls,
  isWindowMaximized,
  onMinimize,
  onToggleMaximize,
  onClose,
  sidebar,
  topBar,
  banner,
  children,
}: AppShellProps) {
  const { t } = useTranslation();

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground selection:bg-primary/30">
      {/* 顶部拖拽区（覆盖整宽），窗口控件靠右 */}
      {(dragBarHeight > 0 || useAppWindowControls) && (
        <div
          className="fixed left-0 right-0 top-0 z-[70] flex items-center justify-end px-2"
          data-tauri-drag-region
          style={{ WebkitAppRegion: "drag", height: dragBarHeight } as any}
        >
          {useAppWindowControls && (
            <div
              className="flex items-center gap-1"
              style={{ WebkitAppRegion: "no-drag" } as any}
            >
              <Button
                variant="ghost"
                size="icon"
                onClick={onMinimize}
                title={t("header.windowMinimize")}
                className="h-7 w-7"
              >
                <Minus className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={onToggleMaximize}
                title={
                  isWindowMaximized
                    ? t("header.windowRestore")
                    : t("header.windowMaximize")
                }
                className="h-7 w-7"
              >
                {isWindowMaximized ? (
                  <Minimize2 className="h-4 w-4" />
                ) : (
                  <Maximize2 className="h-4 w-4" />
                )}
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={onClose}
                title={t("header.windowClose")}
                className="h-7 w-7 hover:bg-red-500/15 hover:text-red-500"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          )}
        </div>
      )}

      {sidebar}

      <div
        className="flex min-w-0 flex-1 flex-col"
        style={{ paddingTop: dragBarHeight }}
      >
        {banner}
        {topBar}
        <main className="flex min-h-0 flex-1 flex-col overflow-y-auto animate-fade-in">
          {children}
        </main>
      </div>
    </div>
  );
}
