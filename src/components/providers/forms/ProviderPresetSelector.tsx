import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { FormLabel } from "@/components/ui/form";
import { ClaudeIcon, CodexIcon, GeminiIcon } from "@/components/BrandIcons";
import {
  Zap,
  Star,
  Layers,
  Settings2,
  ChevronsUpDown,
  Check,
  Box,
} from "lucide-react";
import type { ProviderPreset } from "@/config/claudeProviderPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";
import type { GeminiProviderPreset } from "@/config/geminiProviderPresets";
import type { ClaudeDesktopProviderPreset } from "@/config/claudeDesktopProviderPresets";
import type { OpenCodeProviderPreset } from "@/config/opencodeProviderPresets";
import type { OpenClawProviderPreset } from "@/config/openclawProviderPresets";
import type { HermesProviderPreset } from "@/config/hermesProviderPresets";
import type { ProviderCategory } from "@/types";
import {
  universalProviderPresets,
  type UniversalProviderPreset,
} from "@/config/universalProviderPresets";
import { ProviderIcon } from "@/components/ProviderIcon";
import { hasIcon, isUrlIcon } from "@/icons/extracted";
import { cn } from "@/lib/utils";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";

type AnyPreset =
  | ProviderPreset
  | CodexProviderPreset
  | GeminiProviderPreset
  | ClaudeDesktopProviderPreset
  | OpenCodeProviderPreset
  | OpenClawProviderPreset
  | HermesProviderPreset;

type PresetEntry = {
  id: string;
  preset: AnyPreset;
};

interface ProviderPresetSelectorProps {
  selectedPresetId: string | null;
  presetEntries: PresetEntry[];
  presetCategoryLabels: Record<string, string>;
  onPresetChange: (value: string) => void;
  onUniversalPresetSelect?: (preset: UniversalProviderPreset) => void;
  onManageUniversalProviders?: () => void;
  category?: ProviderCategory; // 当前选中的分类
}

// 下拉内分类分组的固定顺序，未在列表中的分类归入 others
const CATEGORY_ORDER = [
  "official",
  "cn_official",
  "aggregator",
  "third_party",
  "omo",
];

export function ProviderPresetSelector({
  selectedPresetId,
  presetEntries,
  presetCategoryLabels,
  onPresetChange,
  onUniversalPresetSelect,
  onManageUniversalProviders,
  category,
}: ProviderPresetSelectorProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  const getCategoryHint = (): React.ReactNode => {
    switch (category) {
      case "official":
        return t("providerForm.officialHint", {
          defaultValue: "💡 官方供应商使用浏览器登录，无需配置 API Key",
        });
      case "cn_official":
        return t("providerForm.cnOfficialApiKeyHint", {
          defaultValue: "💡 国产官方供应商只需填写 API Key，请求地址已预设",
        });
      case "aggregator":
        return t("providerForm.aggregatorApiKeyHint", {
          defaultValue: "💡 聚合服务供应商只需填写 API Key 即可使用",
        });
      case "third_party":
        return t("providerForm.thirdPartyApiKeyHint", {
          defaultValue: "💡 第三方供应商需要填写 API Key 和请求地址",
        });
      case "custom":
        return t("providerForm.customApiKeyHint", {
          defaultValue: "💡 自定义配置需手动填写所有必要字段",
        });
      case "omo":
        return t("providerForm.omoHint", {
          defaultValue:
            "💡 OMO 配置管理 Agent 模型分配，兼容 oh-my-openagent.jsonc / oh-my-opencode.jsonc",
        });
      default:
        return t("providerPreset.hint", {
          defaultValue: "选择预设后可继续调整下方字段。",
        });
    }
  };

  const presetName = (preset: AnyPreset): string =>
    preset.nameKey ? t(preset.nameKey) : preset.name;

  const renderPresetIcon = (preset: AnyPreset) => {
    // 1. 优先使用预设自带的真实平台图标（与选中后表单展示的一致）
    const iconName = preset.icon;
    if (iconName && (isUrlIcon(iconName) || hasIcon(iconName))) {
      return (
        <ProviderIcon
          icon={iconName}
          name={presetName(preset)}
          size={16}
          className="rounded-[4px]"
          showFallback={false}
        />
      );
    }
    // 2. 主题品牌字形
    switch (preset.theme?.icon) {
      case "claude":
        return <ClaudeIcon size={16} />;
      case "codex":
        return <CodexIcon size={16} />;
      case "gemini":
        return <GeminiIcon size={16} />;
      case "generic":
        return <Zap size={16} />;
      default:
        // 3. 都没有时显示占位图标，后续接入真实图标后自动替换
        return <Box size={16} className="text-muted-foreground" />;
    }
  };

  // 按分类分桶，并按固定顺序排列；组内保持原有顺序
  const groupedEntries = useMemo(() => {
    const buckets = new Map<string, PresetEntry[]>();
    for (const entry of presetEntries) {
      const cat = entry.preset.category ?? "others";
      const list = buckets.get(cat);
      if (list) {
        list.push(entry);
      } else {
        buckets.set(cat, [entry]);
      }
    }
    const ordered: { category: string; entries: PresetEntry[] }[] = [];
    for (const cat of CATEGORY_ORDER) {
      const list = buckets.get(cat);
      if (list) {
        ordered.push({ category: cat, entries: list });
        buckets.delete(cat);
      }
    }
    for (const [cat, entries] of buckets) {
      ordered.push({ category: cat, entries });
    }
    return ordered;
  }, [presetEntries]);

  const isCustom = !selectedPresetId || selectedPresetId === "custom";
  const selectedEntry = isCustom
    ? null
    : presetEntries.find((e) => e.id === selectedPresetId);
  const triggerLabel = selectedEntry
    ? presetName(selectedEntry.preset)
    : t("providerPreset.custom");

  const handleSelect = (value: string) => {
    onPresetChange(value);
    setOpen(false);
  };

  const categoryHeading = (cat: string) =>
    presetCategoryLabels[cat] ?? t("providerPreset.other");

  return (
    <div className="space-y-3">
      <FormLabel>{t("providerPreset.label")}</FormLabel>

      <Popover modal open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <button
            type="button"
            role="combobox"
            aria-expanded={open}
            className="flex h-10 w-full items-center justify-between gap-2 rounded-lg border border-border-default bg-background px-3 py-2 text-sm shadow-sm transition-colors hover:bg-accent focus:outline-none focus-visible:outline-none sm:w-[320px]"
          >
            <span className="flex min-w-0 items-center gap-2 font-medium">
              {selectedEntry ? (
                renderPresetIcon(selectedEntry.preset)
              ) : (
                <Settings2 className="h-3.5 w-3.5 text-muted-foreground" />
              )}
              <span className="truncate">{triggerLabel}</span>
            </span>
            <ChevronsUpDown className="h-4 w-4 shrink-0 opacity-50" />
          </button>
        </PopoverTrigger>
        <PopoverContent
          side="bottom"
          align="start"
          sideOffset={6}
          avoidCollisions
          collisionPadding={8}
          className="z-[1000] w-[var(--radix-popover-trigger-width)] min-w-[280px] p-0 border-border-default"
        >
          <Command>
            <CommandInput
              placeholder={t("providerPreset.searchPlaceholder", {
                defaultValue: "搜索预设…",
              })}
            />
            <CommandList>
              <CommandEmpty>
                {t("providerPreset.empty", {
                  defaultValue: "未找到匹配的预设",
                })}
              </CommandEmpty>

              <CommandGroup>
                <CommandItem
                  value="custom"
                  keywords={[t("providerPreset.custom")]}
                  onSelect={() => handleSelect("custom")}
                >
                  <Check
                    className={cn(
                      "mr-2 h-4 w-4",
                      isCustom ? "opacity-100" : "opacity-0",
                    )}
                  />
                  <Settings2 className="mr-2 h-3.5 w-3.5 text-muted-foreground" />
                  <span className="truncate">{t("providerPreset.custom")}</span>
                </CommandItem>
              </CommandGroup>

              {groupedEntries.map(({ category: cat, entries }) => (
                <CommandGroup key={cat} heading={categoryHeading(cat)}>
                  {entries.map((entry) => {
                    const isSelected = selectedPresetId === entry.id;
                    return (
                      <CommandItem
                        key={entry.id}
                        value={entry.id}
                        keywords={[presetName(entry.preset)]}
                        onSelect={() => handleSelect(entry.id)}
                      >
                        <Check
                          className={cn(
                            "mr-2 h-4 w-4",
                            isSelected ? "opacity-100" : "opacity-0",
                          )}
                        />
                        <span className="mr-2 flex w-4 shrink-0 justify-center">
                          {renderPresetIcon(entry.preset)}
                        </span>
                        <span className="truncate">
                          {presetName(entry.preset)}
                        </span>
                        {entry.preset.isPartner && (
                          <span className="ml-auto flex items-center gap-0.5 rounded-full bg-gradient-to-r from-amber-500 to-yellow-500 px-1.5 py-0.5 text-[10px] font-bold text-white shadow-sm">
                            <Star className="h-2.5 w-2.5 fill-current" />
                          </span>
                        )}
                      </CommandItem>
                    );
                  })}
                </CommandGroup>
              ))}
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>

      {onUniversalPresetSelect && universalProviderPresets.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          {universalProviderPresets.map((preset) => (
            <button
              key={`universal-${preset.providerType}`}
              type="button"
              onClick={() => onUniversalPresetSelect(preset)}
              className="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors bg-accent text-muted-foreground hover:bg-accent/80 relative"
              title={t("universalProvider.hint", {
                defaultValue: "跨应用统一配置，自动同步到 Claude/Codex/Gemini",
              })}
            >
              <ProviderIcon icon={preset.icon} name={preset.name} size={14} />
              {preset.name}
              <span className="absolute -top-1 -right-1 flex items-center gap-0.5 rounded-full bg-gradient-to-r from-indigo-500 to-purple-500 px-1.5 py-0.5 text-[10px] font-bold text-white shadow-md">
                <Layers className="h-2.5 w-2.5" />
              </span>
            </button>
          ))}
          {onManageUniversalProviders && (
            <button
              type="button"
              onClick={onManageUniversalProviders}
              className="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors bg-accent text-muted-foreground hover:bg-accent/80"
              title={t("universalProvider.manage", {
                defaultValue: "管理统一供应商",
              })}
            >
              <Settings2 className="h-4 w-4" />
              {t("universalProvider.manage", {
                defaultValue: "管理",
              })}
            </button>
          )}
        </div>
      )}

      <p className="text-xs text-muted-foreground">{getCategoryHint()}</p>
    </div>
  );
}
