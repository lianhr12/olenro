/**
 * Provider-preset snapshot generator.
 *
 * Not a behavioural test — this evaluates the real TypeScript preset arrays
 * (including the ones whose `settingsConfig`/`config` are built by helper
 * functions and spreads) and serializes them to a JSON snapshot that
 * `olenro-core` embeds via `include_str!`. Run with:
 *
 *   npx vitest run scripts/genProviderPresets.test.ts
 *
 * The committed snapshot at crates/olenro-core/assets/provider_presets.json is
 * the single source of truth shared by the desktop frontend and the Rust core,
 * so the TUI's preset selector stays 1:1 with the desktop app.
 */
import { writeFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { describe, it, expect } from "vitest";

import { providerPresets } from "@/config/claudeProviderPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { geminiProviderPresets } from "@/config/geminiProviderPresets";
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import { openclawProviderPresets } from "@/config/openclawProviderPresets";
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";
import { claudeDesktopProviderPresets } from "@/config/claudeDesktopProviderPresets";
import { universalProviderPresets } from "@/config/universalProviderPresets";

// Keyed by the AppType string used in olenro-core (see provider.rs::AppType).
const snapshot = {
  claude: providerPresets,
  "claude-desktop": claudeDesktopProviderPresets,
  codex: codexProviderPresets,
  gemini: geminiProviderPresets,
  opencode: opencodeProviderPresets,
  openclaw: openclawProviderPresets,
  hermes: hermesProviderPresets,
  universal: universalProviderPresets,
};

describe("provider preset snapshot", () => {
  it("serializes all preset arrays to the core JSON snapshot", () => {
    const outDir = path.resolve(
      __dirname,
      "../crates/olenro-core/assets",
    );
    mkdirSync(outDir, { recursive: true });
    const outFile = path.join(outDir, "provider_presets.json");
    writeFileSync(outFile, JSON.stringify(snapshot, null, 2) + "\n", "utf8");

    // Sanity: every group is a non-empty array (catches accidental import/eval breakage).
    for (const [app, list] of Object.entries(snapshot)) {
      expect(Array.isArray(list), `${app} presets should be an array`).toBe(true);
      expect(list.length, `${app} presets should be non-empty`).toBeGreaterThan(0);
    }
  });
});
