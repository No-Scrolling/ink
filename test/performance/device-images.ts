import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { deflateSync, inflateSync } from "node:zlib";

export const imageCount = 48;
export const imageSize = 512;
export const sha256 = (bytes: Uint8Array | string) => createHash("sha256").update(bytes).digest("hex");
export type Pixels = { width: number; height: number; data: Uint8Array };

function crc32(bytes: Uint8Array) {
  let value = 0xffffffff;
  for (const byte of bytes) {
    value ^= byte;
    for (let bit = 0; bit < 8; bit++) value = (value >>> 1) ^ (value & 1 ? 0xedb88320 : 0);
  }
  return (value ^ 0xffffffff) >>> 0;
}

function chunk(name: string, bytes: Uint8Array) {
  const body = Buffer.concat([Buffer.from(name), bytes]);
  const result = Buffer.alloc(body.length + 8);
  result.writeUInt32BE(bytes.length);
  body.copy(result, 4);
  result.writeUInt32BE(crc32(body), result.length - 4);
  return result;
}

export function encodePng(image: Pixels) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(image.width); header.writeUInt32BE(image.height, 4);
  header[8] = 8; header[9] = 6;
  const rows = Buffer.alloc((image.width * 4 + 1) * image.height);
  for (let y = 0; y < image.height; y++) rows.set(image.data.subarray(y * image.width * 4, (y + 1) * image.width * 4), y * (image.width * 4 + 1) + 1);
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", header), chunk("IDAT", deflateSync(rows)), chunk("IEND", new Uint8Array())]);
}

// Android screencap emits non-interlaced, eight-bit RGB/RGBA PNGs. Reject other formats.
export function decodePng(bytes: Uint8Array): Pixels {
  const input = Buffer.from(bytes);
  if (!input.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) throw Error("Invalid PNG signature");
  let width = 0, height = 0, channels = 0, ended = false;
  const parts: Uint8Array[] = [];
  for (let offset = 8; offset + 12 <= input.length;) {
    const length = input.readUInt32BE(offset), end = offset + 8 + length;
    if (end + 4 > input.length || crc32(input.subarray(offset + 4, end)) !== input.readUInt32BE(end)) throw Error("Invalid PNG chunk");
    const name = input.toString("ascii", offset + 4, offset + 8), data = input.subarray(offset + 8, end);
    if (name === "IHDR") {
      width = data.readUInt32BE(); height = data.readUInt32BE(4);
      if (data[8] !== 8 || ![2, 6].includes(data[9]!) || data[10] || data[11] || data[12]) throw Error("Unsupported screenshot PNG format");
      channels = data[9] === 6 ? 4 : 3;
      if (width < 1 || height < 1 || width * height > 16_000_000) throw Error("Invalid PNG dimensions");
    }
    if (name === "IDAT") parts.push(data);
    if (name === "IEND") { ended = true; break; }
    offset = end + 4;
  }
  if (!ended || !channels) throw Error("Incomplete PNG");
  const stride = width * channels, raw = inflateSync(Buffer.concat(parts), { maxOutputLength: (stride + 1) * height });
  if (raw.length !== (stride + 1) * height) throw Error("Invalid PNG scanlines");
  const scanlines = new Uint8Array(stride * height), rgba = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)]!;
    if (filter > 4) throw Error("Unsupported PNG filter");
    for (let x = 0; x < stride; x++) {
      const left = x >= channels ? scanlines[y * stride + x - channels]! : 0;
      const above = y ? scanlines[(y - 1) * stride + x]! : 0;
      const corner = y && x >= channels ? scanlines[(y - 1) * stride + x - channels]! : 0;
      const prediction = left + above - corner;
      const paeth = Math.abs(prediction - left) <= Math.abs(prediction - above) && Math.abs(prediction - left) <= Math.abs(prediction - corner)
        ? left : Math.abs(prediction - above) <= Math.abs(prediction - corner) ? above : corner;
      const adjustment = [0, left, above, Math.floor((left + above) / 2), paeth][filter]!;
      scanlines[y * stride + x] = raw[y * (stride + 1) + x + 1]! + adjustment;
    }
  }
  for (let pixel = 0; pixel < width * height; pixel++) {
    rgba.set(scanlines.subarray(pixel * channels, pixel * channels + 3), pixel * 4);
    rgba[pixel * 4 + 3] = channels === 4 ? scanlines[pixel * channels + 3]! : 255;
  }
  return { width, height, data: rgba };
}

export function artwork(index: number): Pixels {
  const data = new Uint8Array(imageSize * imageSize * 4);
  for (let y = 0; y < imageSize; y++) for (let x = 0; x < imageSize; x++) {
    // Four broad greys plus a six-bit identity strip survive the e-ink colour transform.
    const shade = y >= 64 && y < 128 && x >= 64 && x < 448
      ? ((index >> Math.floor((x - 64) / 64)) & 1 ? 240 : 16)
      : [32, 96, 160, 224][(y >= 256 ? 2 : 0) + (x >= 256 ? 1 : 0)]!;
    const offset = (y * imageSize + x) * 4;
    data.set([shade, shade, shade, 255], offset);
  }
  return { width: imageSize, height: imageSize, data };
}

export async function prepareImages(fixture: string, directory: string) {
  await mkdir(directory, { recursive: true });
  await mkdir(resolve(fixture, ".ink"), { recursive: true });
  const manifest = [];
  for (let index = 0; index < imageCount; index++) {
    const bytes = encodePng(artwork(index)), file = `${index}.png`;
    await writeFile(resolve(directory, file), bytes);
    manifest.push({ index, file, sha256: sha256(bytes), bytes: bytes.length, width: imageSize, height: imageSize });
  }
  await writeFile(resolve(fixture, ".ink/device-images.ts"), manifest.map(({ index }) => `import image${index} from ${JSON.stringify(resolve(directory, `${index}.png`))};`).join("\n")
    + `\nexport const images = [${manifest.map(({ index }) => `image${index}`).join(", ")}];\n`);
  return manifest;
}

function artworkPixelsAt(image: Pixels, index: number, x: number, y: number, requireWholeImage: boolean) {
  const probes = [
    { x: 24, y: 152, width: 208, height: 80, expected: 32 },
    { x: 280, y: 152, width: 208, height: 80, expected: 96 },
    ...(requireWholeImage ? [{ x: 24, y: 280, width: 208, height: 208, expected: 160 }, { x: 280, y: 280, width: 208, height: 208, expected: 224 }] : []),
    ...Array.from({ length: 6 }, (_, bit) => ({ x: 72 + bit * 64, y: 72, width: 48, height: 48, expected: index & 2 ** bit ? 240 : 16 })),
  ];
  let comparedPixels = 0, differingPixels = 0, maximumChannelDifference = 0;
  for (const probe of probes) for (let py = 0; py < probe.height; py++) for (let px = 0; px < probe.width; px++) {
    const offset = ((y + probe.y + py) * image.width + x + probe.x + px) * 4;
    let different = false;
    for (let channel = 0; channel < 3; channel++) {
      const delta = Math.abs(image.data[offset + channel]! - probe.expected);
      maximumChannelDifference = Math.max(maximumChannelDifference, delta);
      different ||= delta > 2;
    }
    different ||= image.data[offset + 3] !== 255;
    comparedPixels++; if (different) differingPixels++;
  }
  if (!differingPixels) return { index, x, y, width: 512, height: requireWholeImage ? 512 : 256, comparedPixels, differingPixels, maximumChannelDifference, tolerance: 2 };
  return undefined;
}

export function verifyArtwork(image: Pixels, index: number, requireWholeImage = true) {
  const shadeAt = (x: number, y: number) => image.data[(y * image.width + x) * 4];
  // Locate the upper-left 32-grey quadrant, then check independently authored probes.
  for (let y = 0; y <= image.height - (requireWholeImage ? 512 : 256); y++) for (let x = 0; x <= image.width - 512; x++) {
    if (Math.abs((shadeAt(x + 32, y + 32) ?? -100) - 32) > 2 || Math.abs((shadeAt(x + 300, y + 32) ?? -100) - 96) > 2) continue;
    if (Math.abs((shadeAt(x, y + 32) ?? -100) - 32) > 2 || Math.abs((shadeAt(x + 32, y) ?? -100) - 32) > 2) continue;
    if (x && Math.abs((shadeAt(x - 1, y + 32) ?? 0) - 32) <= 2) continue;
    if (y && Math.abs((shadeAt(x + 32, y - 1) ?? 0) - 32) <= 2) continue;
    const compared = artworkPixelsAt(image, index, x, y, requireWholeImage);
    if (compared) return compared;
  }
  throw Error(`Saved screenshot does not contain the expected 512-pixel artwork ${index}`);
}

export function verifyAnyArtwork(image: Pixels) {
  for (let y = 0; y <= image.height - 512; y++) for (let x = 0; x <= image.width - 512; x++) {
    const at = (px: number, py: number) => image.data[((y + py) * image.width + x + px) * 4]!;
    if (Math.abs(at(32, 32) - 32) > 2 || Math.abs(at(300, 32) - 96) > 2 || Math.abs(at(32, 300) - 160) > 2) continue;
    if (Math.abs(at(0, 32) - 32) > 2 || Math.abs(at(32, 0) - 32) > 2) continue;
    if (x && Math.abs(image.data[((y + 32) * image.width + x - 1) * 4]! - 32) <= 2) continue;
    if (y && Math.abs(image.data[((y - 1) * image.width + x + 32) * 4]! - 32) <= 2) continue;
    let index = 0;
    for (let bit = 0; bit < 6; bit++) if (at(96 + bit * 64, 96) > 128) index += 2 ** bit;
    if (index >= imageCount) continue;
    const compared = artworkPixelsAt(image, index, x, y, true);
    if (compared) return compared;
  }
  throw Error("Saved scrolling screenshot contains no complete, recognised artwork");
}
