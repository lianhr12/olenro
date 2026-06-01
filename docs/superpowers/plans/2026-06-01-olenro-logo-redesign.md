# Olenro Logo Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current CC Switch-inspired app logo with an original, open-source-safe minimal broken-ring `O` mark across all app and packaging icon assets.

**Architecture:** Keep one source of truth in `assets/brand/olenro-logo.svg`, then use the existing Tauri CLI icon generator to regenerate platform assets into `src-tauri/icons/`. Add a small Node verification script that checks expected files, key image dimensions, ICO/ICNS signatures, and that the two primary old icon hashes are no longer present.

**Tech Stack:** Tauri CLI `pnpm tauri icon`, SVG, Node.js built-ins, macOS `sips`, existing React/Vite/Tauri app.

---

## File Map

- Create: `assets/brand/olenro-logo.svg`  
  Original vector master for the new broken-ring Olenro `O` mark.
- Create: `scripts/verify-logo-assets.mjs`  
  Repo-local asset verification script using Node built-ins only.
- Modify: `src-tauri/icons/*`  
  Generated desktop, Windows, Android, and iOS icon PNG/ICO/ICNS files consumed by Tauri packaging.
- Modify: `src/assets/icons/app-icon.png`  
  32px in-app icon used by `src/components/settings/AboutSection.tsx`.
- Do not modify: `src-tauri/icons/tray/macos/*`  
  Tray status icons are monochrome status assets and are out of scope.
- Do not modify: `src-tauri/icons/dmg-background.png`  
  The DMG background is not the logo.
- Do not modify: `tests/components/SkillsPageInstall.test.tsx` or `tests/components/UnifiedSkillsPanel.test.tsx`  
  These files already had unrelated working-tree changes before this plan.

---

### Task 1: Add Logo Asset Verification

**Files:**
- Create: `scripts/verify-logo-assets.mjs`

- [ ] **Step 1: Create the verification script**

Add `scripts/verify-logo-assets.mjs` with this exact content:

```js
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";

const oldHashes = new Map([
  [
    "src-tauri/icons/icon.png",
    "04225b1b9c54569ec1ec850ad9f1c9f33ca4f286dab001a3392c0460deb342e5",
  ],
  [
    "src/assets/icons/app-icon.png",
    "8c167919ba52ed97aaa5b612819821a0c6dba2c7cd2c03a09df0a2086451d2aa",
  ],
]);

const requiredFiles = [
  "assets/brand/olenro-logo.svg",
  "src-tauri/icons/32x32.png",
  "src-tauri/icons/64x64.png",
  "src-tauri/icons/128x128.png",
  "src-tauri/icons/128x128@2x.png",
  "src-tauri/icons/icon.png",
  "src-tauri/icons/icon.ico",
  "src-tauri/icons/icon.icns",
  "src-tauri/icons/Square30x30Logo.png",
  "src-tauri/icons/Square44x44Logo.png",
  "src-tauri/icons/Square71x71Logo.png",
  "src-tauri/icons/Square89x89Logo.png",
  "src-tauri/icons/Square107x107Logo.png",
  "src-tauri/icons/Square142x142Logo.png",
  "src-tauri/icons/Square150x150Logo.png",
  "src-tauri/icons/Square284x284Logo.png",
  "src-tauri/icons/Square310x310Logo.png",
  "src-tauri/icons/StoreLogo.png",
  "src-tauri/icons/android/mipmap-mdpi/ic_launcher.png",
  "src-tauri/icons/android/mipmap-mdpi/ic_launcher_foreground.png",
  "src-tauri/icons/android/mipmap-mdpi/ic_launcher_round.png",
  "src-tauri/icons/android/mipmap-hdpi/ic_launcher.png",
  "src-tauri/icons/android/mipmap-hdpi/ic_launcher_foreground.png",
  "src-tauri/icons/android/mipmap-hdpi/ic_launcher_round.png",
  "src-tauri/icons/android/mipmap-xhdpi/ic_launcher.png",
  "src-tauri/icons/android/mipmap-xhdpi/ic_launcher_foreground.png",
  "src-tauri/icons/android/mipmap-xhdpi/ic_launcher_round.png",
  "src-tauri/icons/android/mipmap-xxhdpi/ic_launcher.png",
  "src-tauri/icons/android/mipmap-xxhdpi/ic_launcher_foreground.png",
  "src-tauri/icons/android/mipmap-xxhdpi/ic_launcher_round.png",
  "src-tauri/icons/android/mipmap-xxxhdpi/ic_launcher.png",
  "src-tauri/icons/android/mipmap-xxxhdpi/ic_launcher_foreground.png",
  "src-tauri/icons/android/mipmap-xxxhdpi/ic_launcher_round.png",
  "src-tauri/icons/ios/AppIcon-20x20@1x.png",
  "src-tauri/icons/ios/AppIcon-20x20@2x.png",
  "src-tauri/icons/ios/AppIcon-20x20@2x-1.png",
  "src-tauri/icons/ios/AppIcon-20x20@3x.png",
  "src-tauri/icons/ios/AppIcon-29x29@1x.png",
  "src-tauri/icons/ios/AppIcon-29x29@2x.png",
  "src-tauri/icons/ios/AppIcon-29x29@2x-1.png",
  "src-tauri/icons/ios/AppIcon-29x29@3x.png",
  "src-tauri/icons/ios/AppIcon-40x40@1x.png",
  "src-tauri/icons/ios/AppIcon-40x40@2x.png",
  "src-tauri/icons/ios/AppIcon-40x40@2x-1.png",
  "src-tauri/icons/ios/AppIcon-40x40@3x.png",
  "src-tauri/icons/ios/AppIcon-60x60@2x.png",
  "src-tauri/icons/ios/AppIcon-60x60@3x.png",
  "src-tauri/icons/ios/AppIcon-76x76@1x.png",
  "src-tauri/icons/ios/AppIcon-76x76@2x.png",
  "src-tauri/icons/ios/AppIcon-83.5x83.5@2x.png",
  "src-tauri/icons/ios/AppIcon-512@2x.png",
  "src/assets/icons/app-icon.png",
];

const pngDimensions = new Map([
  ["src-tauri/icons/32x32.png", [32, 32]],
  ["src-tauri/icons/64x64.png", [64, 64]],
  ["src-tauri/icons/128x128.png", [128, 128]],
  ["src-tauri/icons/128x128@2x.png", [256, 256]],
  ["src-tauri/icons/icon.png", [512, 512]],
  ["src-tauri/icons/Square310x310Logo.png", [310, 310]],
  ["src-tauri/icons/ios/AppIcon-512@2x.png", [1024, 1024]],
  ["src/assets/icons/app-icon.png", [32, 32]],
]);

const failures = [];

for (const file of requiredFiles) {
  if (!existsSync(file)) {
    failures.push(`Missing file: ${file}`);
  }
}

for (const [file, oldHash] of oldHashes) {
  if (!existsSync(file)) continue;
  const hash = createHash("sha256").update(readFileSync(file)).digest("hex");
  if (hash === oldHash) {
    failures.push(`Icon still matches old CC Switch-inspired asset: ${file}`);
  }
}

const svgPath = "assets/brand/olenro-logo.svg";
if (existsSync(svgPath)) {
  const svg = readFileSync(svgPath, "utf8");
  if (!svg.includes('viewBox="0 0 1024 1024"')) {
    failures.push(`${svgPath} must use viewBox="0 0 1024 1024"`);
  }
  if (!svg.includes("Broken-ring Olenro O logo")) {
    failures.push(`${svgPath} must include the expected title`);
  }
}

function readPngDimensions(file) {
  const buffer = readFileSync(file);
  const signature = buffer.subarray(0, 8).toString("hex");
  if (signature !== "89504e470d0a1a0a") {
    throw new Error(`${file} is not a PNG`);
  }
  return [buffer.readUInt32BE(16), buffer.readUInt32BE(20)];
}

for (const [file, expected] of pngDimensions) {
  if (!existsSync(file)) continue;
  const [width, height] = readPngDimensions(file);
  if (width !== expected[0] || height !== expected[1]) {
    failures.push(
      `${file} expected ${expected[0]}x${expected[1]}, got ${width}x${height}`,
    );
  }
}

if (existsSync("src-tauri/icons/icon.ico")) {
  const ico = readFileSync("src-tauri/icons/icon.ico");
  if (ico.readUInt16LE(0) !== 0 || ico.readUInt16LE(2) !== 1) {
    failures.push("src-tauri/icons/icon.ico does not have an ICO signature");
  }
}

if (existsSync("src-tauri/icons/icon.icns")) {
  const icns = readFileSync("src-tauri/icons/icon.icns");
  if (icns.subarray(0, 4).toString("ascii") !== "icns") {
    failures.push("src-tauri/icons/icon.icns does not have an ICNS signature");
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log(`Verified ${requiredFiles.length} logo asset files.`);
```

- [ ] **Step 2: Run the verification script and confirm it fails for the missing new SVG**

Run:

```bash
node scripts/verify-logo-assets.mjs
```

Expected: command exits non-zero and includes:

```text
Missing file: assets/brand/olenro-logo.svg
Icon still matches old CC Switch-inspired asset: src-tauri/icons/icon.png
Icon still matches old CC Switch-inspired asset: src/assets/icons/app-icon.png
```

- [ ] **Step 3: Commit the verification script**

Run:

```bash
git add scripts/verify-logo-assets.mjs
git commit -m "Add logo asset verification"
```

Expected: one commit containing only `scripts/verify-logo-assets.mjs`.

---

### Task 2: Add Original SVG Source and Generate Icons

**Files:**
- Create: `assets/brand/olenro-logo.svg`
- Modify: `src-tauri/icons/*.png`
- Modify: `src-tauri/icons/*.ico`
- Modify: `src-tauri/icons/*.icns`
- Modify: `src-tauri/icons/android/mipmap-*/*.png`
- Modify: `src-tauri/icons/ios/*.png`
- Modify: `src/assets/icons/app-icon.png`

- [ ] **Step 1: Add the SVG master source**

Create `assets/brand/olenro-logo.svg` with this exact content:

```svg
<svg width="1024" height="1024" viewBox="0 0 1024 1024" fill="none" xmlns="http://www.w3.org/2000/svg">
  <title>Broken-ring Olenro O logo</title>
  <rect x="96" y="96" width="832" height="832" rx="212" fill="#111827"/>
  <circle cx="512" cy="512" r="278" stroke="#F8FAFC" stroke-width="116" stroke-linecap="round" stroke-dasharray="1320 430" transform="rotate(-42 512 512)"/>
  <path d="M666 232H812V378" stroke="#22D3EE" stroke-width="92" stroke-linecap="round" stroke-linejoin="round"/>
  <circle cx="512" cy="512" r="124" fill="#111827"/>
</svg>
```

- [ ] **Step 2: Generate the Tauri icon set from the SVG**

Run:

```bash
pnpm tauri icon assets/brand/olenro-logo.svg --output src-tauri/icons --ios-color "#111827"
```

Expected: command exits 0 and regenerates files in `src-tauri/icons/`.

- [ ] **Step 3: Update the in-app 32px icon**

Run:

```bash
cp src-tauri/icons/32x32.png src/assets/icons/app-icon.png
```

Expected: `src/assets/icons/app-icon.png` becomes a 32x32 PNG matching the new generated logo.

- [ ] **Step 4: Verify out-of-scope assets did not change**

Run:

```bash
git diff --stat -- src-tauri/icons/tray/macos src-tauri/icons/dmg-background.png
```

Expected: no output.

- [ ] **Step 5: Run logo asset verification**

Run:

```bash
node scripts/verify-logo-assets.mjs
```

Expected:

```text
Verified 52 logo asset files.
```

- [ ] **Step 6: Inspect key generated formats**

Run:

```bash
file src-tauri/icons/icon.png src-tauri/icons/icon.ico src-tauri/icons/icon.icns src/assets/icons/app-icon.png
```

Expected output includes:

```text
src-tauri/icons/icon.png:       PNG image data, 512 x 512
src-tauri/icons/icon.ico:       MS Windows icon resource
src-tauri/icons/icon.icns:      Mac OS X icon
src/assets/icons/app-icon.png:  PNG image data, 32 x 32
```

- [ ] **Step 7: Visually inspect the 512px and 32px icons**

Use the local image viewer tool or OS preview to inspect:

```text
src-tauri/icons/icon.png
src/assets/icons/app-icon.png
```

Expected: both show a minimal graphite broken-ring `O` with a cyan top-right accent. The 32px icon must not collapse into an unreadable blob.

- [ ] **Step 8: Commit generated icon assets**

Run:

```bash
git add assets/brand/olenro-logo.svg src-tauri/icons src/assets/icons/app-icon.png
git commit -m "Replace app logo with Olenro mark"
```

Expected: one commit containing the SVG source and generated icon assets. It must not include `src-tauri/icons/tray/macos/*`, `src-tauri/icons/dmg-background.png`, `tests/components/SkillsPageInstall.test.tsx`, or `tests/components/UnifiedSkillsPanel.test.tsx`.

---

### Task 3: Final Verification

**Files:**
- No new files
- Verify: `scripts/verify-logo-assets.mjs`
- Verify: `src-tauri/tauri.conf.json`
- Verify: `src/components/settings/AboutSection.tsx`

- [ ] **Step 1: Run asset verification again**

Run:

```bash
node scripts/verify-logo-assets.mjs
```

Expected:

```text
Verified 52 logo asset files.
```

- [ ] **Step 2: Run TypeScript typecheck**

Run:

```bash
pnpm run typecheck
```

Expected: command exits 0.

- [ ] **Step 3: Confirm Tauri config still points to existing icon paths**

Run:

```bash
rg -n '"icons/32x32.png"|"icons/128x128.png"|"icons/128x128@2x.png"|"icons/icon.icns"|"icons/icon.ico"' src-tauri/tauri.conf.json
```

Expected output includes all five configured bundle icon paths:

```text
src-tauri/tauri.conf.json:41:      "icons/32x32.png",
src-tauri/tauri.conf.json:42:      "icons/128x128.png",
src-tauri/tauri.conf.json:43:      "icons/128x128@2x.png",
src-tauri/tauri.conf.json:44:      "icons/icon.icns",
src-tauri/tauri.conf.json:45:      "icons/icon.ico"
```

- [ ] **Step 4: Confirm the About page still imports the app icon**

Run:

```bash
rg -n 'app-icon.png|<img src=\{appIcon\}' src/components/settings/AboutSection.tsx
```

Expected output includes:

```text
src/components/settings/AboutSection.tsx:38:import appIcon from "@/assets/icons/app-icon.png";
src/components/settings/AboutSection.tsx:773:              <img src={appIcon} alt="Olenro" className="h-5 w-5" />
```

- [ ] **Step 5: Confirm unrelated working-tree changes remain unstaged**

Run:

```bash
git status --short
```

Expected: any remaining changes to `tests/components/SkillsPageInstall.test.tsx` and `tests/components/UnifiedSkillsPanel.test.tsx` are shown as unstaged user changes, not included in the logo commits.
