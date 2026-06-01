# Olenro Logo Redesign

## Goal

Replace the current CC Switch-inspired application logo with an original Olenro brand mark that is suitable for open-source distribution with the project.

## Direction

Use a minimal broken-ring `O` mark. The shape should be a clean geometric ring with a deliberate break or angled opening, signaling a distinct Olenro identity while lightly suggesting switching, routing, or an exit path. The mark must not reuse the current radial/starburst composition, color layout, or silhouette.

## Visual Requirements

- Primary symbol: abstract `O`, not a wordmark.
- Style: minimal, restrained, and recognizable at small desktop icon sizes.
- Color: avoid the current orange/yellow/teal radial palette. Prefer a compact palette such as deep graphite plus a single cool accent.
- Shape: clear on transparent, light, and dark backgrounds.
- Licensing: created in-repo from original vector geometry, with no external logo, trademark, AI provider mark, or stock asset dependency.

## Asset Scope

The implementation should update the icon sources used by the app and packaging:

- `src-tauri/icons/icon.png`
- Tauri bundle icons in `src-tauri/icons/`, including PNG sizes, `.ico`, and `.icns`
- Windows square logo PNGs already present in `src-tauri/icons/`
- Android and iOS app icon PNGs already present in `src-tauri/icons/`
- `src/assets/icons/app-icon.png` for the in-app About section

Tray status icons and the DMG background are out of scope unless they visibly reuse the app logo.

## Implementation Approach

Create or use a single SVG master source for the broken-ring `O`, then generate the existing raster icon outputs from that source. Keep filenames and Tauri config unchanged so packaging continues to consume the same paths.

## Verification

- Confirm all expected icon files are regenerated.
- Confirm `file` reports sensible dimensions and formats for key PNG, ICO, and ICNS assets.
- Run the renderer typecheck if available.
- Inspect the 512px icon and 32px app icon visually for clarity.

