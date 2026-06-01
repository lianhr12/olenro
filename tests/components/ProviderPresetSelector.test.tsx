import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi, beforeAll } from "vitest";
import { useForm } from "react-hook-form";
import { Form } from "@/components/ui/form";
import { ProviderPresetSelector } from "@/components/providers/forms/ProviderPresetSelector";

// cmdk 在挂载/高亮时调用 scrollIntoView，jsdom 未实现，需打桩
beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
});

const renderSelector = (onPresetChange = vi.fn()) => {
  const Wrapper = () => {
    const form = useForm();
    return (
      <Form {...form}>
        <ProviderPresetSelector
          selectedPresetId="custom"
          presetEntries={[
            {
              id: "preset-0",
              preset: {
                name: "First",
                websiteUrl: "https://first.example.com",
                settingsConfig: {},
                category: "third_party",
              },
            },
            {
              id: "preset-1",
              preset: {
                name: "Second",
                websiteUrl: "https://second.example.com",
                settingsConfig: {},
                category: "official",
              },
            },
            {
              id: "preset-2",
              preset: {
                name: "Third",
                websiteUrl: "https://third.example.com",
                settingsConfig: {},
                category: "aggregator",
              },
            },
            {
              id: "preset-3",
              preset: {
                name: "Fourth",
                websiteUrl: "https://fourth.example.com",
                settingsConfig: {},
                category: "official",
              },
            },
          ]}
          presetCategoryLabels={{
            official: "官方",
            aggregator: "聚合服务",
            third_party: "第三方",
          }}
          onPresetChange={onPresetChange}
        />
      </Form>
    );
  };
  return render(<Wrapper />);
};

describe("ProviderPresetSelector", () => {
  it("默认触发器显示当前选中（自定义）", () => {
    renderSelector();
    expect(screen.getByRole("combobox").textContent).toContain(
      "providerPreset.custom",
    );
  });

  it("打开下拉后按固定分类顺序分组渲染预设", async () => {
    renderSelector();

    fireEvent.click(screen.getByRole("combobox"));

    // 自定义项 + 各预设项均为 option；顺序按 CATEGORY_ORDER：
    // official(Second, Fourth) → aggregator(Third) → third_party(First)
    const options = await screen.findAllByRole("option");
    expect(options.map((o) => o.textContent?.trim())).toEqual([
      "providerPreset.custom",
      "Second",
      "Fourth",
      "Third",
      "First",
    ]);
  });

  it("选择预设回调传入对应的预设 id", async () => {
    const onPresetChange = vi.fn();
    renderSelector(onPresetChange);

    fireEvent.click(screen.getByRole("combobox"));
    const option = await screen.findByText("Third");
    fireEvent.click(option);

    expect(onPresetChange).toHaveBeenCalledWith("preset-2");
  });
});
