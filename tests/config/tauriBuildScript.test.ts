import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("Tauri build script", () => {
  it("reruns when app icon inputs change", () => {
    const buildScript = readFileSync(
      resolve(process.cwd(), "src-tauri/build.rs"),
      "utf8",
    );

    const watchedInputs = [
      "tauri.conf.json",
      "icons/icon.icns",
      "icons/icon.ico",
      "icons/icon.png",
      "icons/tray/macos/statusTemplate.png",
      "icons/tray/macos/statusTemplate@2x.png",
      "icons/tray/macos/statusbar_template_3x.png",
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
    ];

    expect(buildScript).toContain('println!("cargo:rerun-if-changed={input}");');

    for (const input of watchedInputs) {
      expect(buildScript).toContain(`"${input}"`);
    }
  });
});
