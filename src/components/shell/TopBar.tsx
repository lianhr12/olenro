import type { ReactNode } from "react";
import { DRAG_REGION_ATTR, DRAG_REGION_STYLE } from "@/lib/platform";

const TOPBAR_HEIGHT = 64; // px

interface TopBarProps {
  title: string;
  /** 状态药丸（代理接管 / 故障转移等） */
  status?: ReactNode;
  /** 视图相关操作按钮（添加 / 导入 / 刷新等） */
  actions?: ReactNode;
}

export function TopBar({ title, status, actions }: TopBarProps) {
  return (
    <header
      className="flex shrink-0 items-center justify-between gap-2 px-6"
      {...DRAG_REGION_ATTR}
      style={{ ...DRAG_REGION_STYLE, height: TOPBAR_HEIGHT } as any}
    >
      <h1 className="min-w-0 truncate text-2xl font-bold tracking-tight">
        {title}
      </h1>
      <div
        className="flex shrink-0 items-center gap-2"
        style={{ WebkitAppRegion: "no-drag" } as any}
      >
        {status}
        {actions}
      </div>
    </header>
  );
}
