import { createHash } from "node:crypto";
import { PNG } from "pngjs";

export type Region = { name: string; x: number; y: number; width: number; height: number; meaning: string };
export type ImagePixels = { width: number; height: number; data: Uint8Array };
export const comparisonTolerance = { maxChannelDifference: 16, maxChangedPixelFraction: 0.001 };
export const sha256 = (bytes: Uint8Array | string) => createHash("sha256").update(bytes).digest("hex");
export function decode(bytes: Uint8Array): ImagePixels { return PNG.sync.read(Buffer.from(bytes)); }
export function regionPixels(image: ImagePixels, region: Region): Uint8Array {
  if (![region.x, region.y, region.width, region.height].every(Number.isInteger)
    || region.x < 0 || region.y < 0 || region.width < 1 || region.height < 1
    || region.x + region.width > image.width || region.y + region.height > image.height) throw Error(`Invalid region ${region.name}`);
  const bytes = new Uint8Array(region.width * region.height * 4);
  for (let y = 0; y < region.height; y++) {
    const offset = ((region.y + y) * image.width + region.x) * 4;
    bytes.set(image.data.subarray(offset, offset + region.width * 4), y * region.width * 4);
  }
  return bytes;
}
export function inspect(image: ImagePixels, region: Region) {
  const bytes = regionPixels(image, region);
  const histogram = new Map<string, number>();
  for (let offset = 0; offset < bytes.length; offset += 4) {
    const colour = [...bytes.subarray(offset, offset + 4)].join(",");
    histogram.set(colour, (histogram.get(colour) ?? 0) + 1);
  }
  return { ...region, rgbaSha256: sha256(bytes), distinctColours: histogram.size,
    mostCommonColours: [...histogram].sort((a, b) => b[1] - a[1]).slice(0, 6).map(([rgba, pixels]) => ({ rgba, pixels })) };
}
export function compare(actual: ImagePixels, expected: ImagePixels, region: Region) {
  const left = regionPixels(actual, region), right = regionPixels(expected, region);
  let changedPixels = 0, maxChannelDifference = 0, maxAlphaDifference = 0;
  for (let offset = 0; offset < left.length; offset += 4) {
    let changed = false;
    for (let channel = 0; channel < 4; channel++) {
      const difference = Math.abs(left[offset + channel]! - right[offset + channel]!);
      maxChannelDifference = Math.max(maxChannelDifference, difference);
      if (channel === 3) maxAlphaDifference = Math.max(maxAlphaDifference, difference);
      changed ||= difference !== 0;
    }
    if (changed) changedPixels++;
  }
  return { name: region.name, changedPixels, totalPixels: region.width * region.height, maxChannelDifference, maxAlphaDifference };
}

export function matches(result: ReturnType<typeof compare>) {
  return result.maxAlphaDifference === 0
    && result.maxChannelDifference <= comparisonTolerance.maxChannelDifference
    && result.changedPixels <= Math.floor(result.totalPixels * comparisonTolerance.maxChangedPixelFraction);
}
