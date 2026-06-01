import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function sha256(path: string): string {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function pngDimensions(path: string): [number, number] {
  const buffer = readFileSync(path);
  expect(buffer.subarray(0, 8).toString("hex")).toBe("89504e470d0a1a0a");
  return [buffer.readUInt32BE(16), buffer.readUInt32BE(20)];
}

describe("tray icon configuration", () => {
  it("uses Olenro branding and platform-specific tray icon sources", () => {
    const libRs = readFileSync(
      resolve(process.cwd(), "src-tauri/src/lib.rs"),
      "utf8",
    );

    expect(libRs).toContain('include_bytes!("../icons/tray/macos/statusbar_template_3x.png")');
    expect(libRs).toContain('.tooltip("Olenro")');
    expect(libRs).toContain("tray_builder.icon(icon).icon_as_template(true)");
    expect(libRs).toContain("app.default_window_icon()");
  });

  it("uses regenerated macOS tray template assets", () => {
    const oldHashes = new Set([
      "65c365e7c32929807deec2d7469eed2d5c06f21ce86854a4848057d2df588ab8",
      "737004c91a766a67e57a62f695a28e2390a6fbd48efcb1d3eda1fe5bf289c3f4",
      "c82ff0ffb2800fbcbbdd88b92d1844527ef5fb40eb5d6fb3213bab505b2ee007",
    ]);
    const trayAssets = [
      ["src-tauri/icons/tray/macos/statusTemplate.png", [24, 24]],
      ["src-tauri/icons/tray/macos/statusTemplate@2x.png", [48, 48]],
      ["src-tauri/icons/tray/macos/statusbar_template_3x.png", [72, 72]],
    ] as const;

    for (const [asset, dimensions] of trayAssets) {
      expect(pngDimensions(resolve(process.cwd(), asset))).toEqual(dimensions);
      expect(oldHashes.has(sha256(resolve(process.cwd(), asset)))).toBe(false);
    }
  });
});
