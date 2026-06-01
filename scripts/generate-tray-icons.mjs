import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const OUTPUTS = [
  ["src-tauri/icons/tray/macos/statusTemplate.png", 24],
  ["src-tauri/icons/tray/macos/statusTemplate@2x.png", 48],
  ["src-tauri/icons/tray/macos/statusbar_template_3x.png", 72],
];

const CANVAS = 72;
const SUPERSAMPLE = 4;

const crcTable = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k += 1) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  return c >>> 0;
});

function crc32(buffer) {
  let c = 0xffffffff;
  for (const byte of buffer) {
    c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type, "ascii");
  const payload = Buffer.concat([typeBuffer, data]);
  const output = Buffer.alloc(12 + data.length);
  output.writeUInt32BE(data.length, 0);
  typeBuffer.copy(output, 4);
  data.copy(output, 8);
  output.writeUInt32BE(crc32(payload), 8 + data.length);
  return output;
}

function distanceToSegment(px, py, ax, ay, bx, by) {
  const dx = bx - ax;
  const dy = by - ay;
  const lengthSquared = dx * dx + dy * dy;
  const t =
    lengthSquared === 0
      ? 0
      : Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / lengthSquared));
  const x = ax + t * dx;
  const y = ay + t * dy;
  return Math.hypot(px - x, py - y);
}

function isMarked(x, y) {
  const dx = x - 36;
  const dy = y - 36;
  const radius = Math.hypot(dx, dy);
  const angle = Math.atan2(dy, dx);
  const gap = angle > -1.05 && angle < -0.05;
  const ring = Math.abs(radius - 20) <= 5 && !gap;

  const cornerStroke = 4;
  const corner =
    distanceToSegment(x, y, 47, 16, 58, 16) <= cornerStroke ||
    distanceToSegment(x, y, 58, 16, 58, 27) <= cornerStroke;

  return ring || corner;
}

function render(size) {
  const rgba = Buffer.alloc(size * size * 4);
  const scale = CANVAS / size;

  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      let coverage = 0;
      for (let sy = 0; sy < SUPERSAMPLE; sy += 1) {
        for (let sx = 0; sx < SUPERSAMPLE; sx += 1) {
          const sampleX = (x + (sx + 0.5) / SUPERSAMPLE) * scale;
          const sampleY = (y + (sy + 0.5) / SUPERSAMPLE) * scale;
          if (isMarked(sampleX, sampleY)) coverage += 1;
        }
      }

      const offset = (y * size + x) * 4;
      rgba[offset] = 0;
      rgba[offset + 1] = 0;
      rgba[offset + 2] = 0;
      rgba[offset + 3] = Math.round((coverage / (SUPERSAMPLE * SUPERSAMPLE)) * 255);
    }
  }

  return rgba;
}

function encodePng(size, rgba) {
  const signature = Buffer.from("89504e470d0a1a0a", "hex");
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const rows = Buffer.alloc((size * 4 + 1) * size);
  for (let y = 0; y < size; y += 1) {
    const rowOffset = y * (size * 4 + 1);
    rows[rowOffset] = 0;
    rgba.copy(rows, rowOffset + 1, y * size * 4, (y + 1) * size * 4);
  }

  return Buffer.concat([
    signature,
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(rows, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

for (const [path, size] of OUTPUTS) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, encodePng(size, render(size)));
  console.log(`Generated ${path} (${size}x${size})`);
}
