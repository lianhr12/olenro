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
  [
    "src-tauri/icons/tray/macos/statusTemplate.png",
    "65c365e7c32929807deec2d7469eed2d5c06f21ce86854a4848057d2df588ab8",
  ],
  [
    "src-tauri/icons/tray/macos/statusTemplate@2x.png",
    "737004c91a766a67e57a62f695a28e2390a6fbd48efcb1d3eda1fe5bf289c3f4",
  ],
  [
    "src-tauri/icons/tray/macos/statusbar_template_3x.png",
    "c82ff0ffb2800fbcbbdd88b92d1844527ef5fb40eb5d6fb3213bab505b2ee007",
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
  "src-tauri/icons/tray/macos/statusTemplate.png",
  "src-tauri/icons/tray/macos/statusTemplate@2x.png",
  "src-tauri/icons/tray/macos/statusbar_template_3x.png",
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
  ["src-tauri/icons/tray/macos/statusTemplate.png", [24, 24]],
  ["src-tauri/icons/tray/macos/statusTemplate@2x.png", [48, 48]],
  ["src-tauri/icons/tray/macos/statusbar_template_3x.png", [72, 72]],
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
