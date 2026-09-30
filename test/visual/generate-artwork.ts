import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";

const digits = [
  ["111", "101", "101", "101", "111"], ["010", "110", "010", "010", "111"],
  ["111", "001", "111", "100", "111"], ["111", "001", "111", "001", "111"],
  ["101", "101", "111", "001", "001"], ["111", "100", "111", "001", "111"],
  ["111", "100", "111", "101", "111"], ["111", "001", "010", "010", "010"],
  ["111", "101", "111", "101", "111"], ["111", "101", "111", "001", "111"],
];

export async function generateArtwork(fixture: string) {
  const directory = resolve(fixture, ".ink/artwork");
  await mkdir(directory, { recursive: true });
  const imports: string[] = [];
  for (let row = 1; row <= 32; row++) {
    let seed = row;
    const random = (limit: number) => {
      seed ^= seed << 13; seed ^= seed >>> 17; seed ^= seed << 5;
      return (seed >>> 0) % limit;
    };
    const start = 40 + random(170), end = 40 + random(170), accent = 40 + random(190);
    const circles = Array.from({ length: 8 }, () => ({ x: random(300) - 22, y: random(300) - 22,
      radius: 15 + random(60), stroke: 2 + random(7) }));
    const image = new PNG({ width: 256, height: 256 });
    const label = String(row).padStart(2, "0");
    for (let y = 0; y < 256; y++) for (let x = 0; x < 256; x++) {
      let shade = Math.round(start + (end - start) * y / 255);
      if (circles.some(circle => Math.abs(Math.hypot(x - circle.x, y - circle.y) - circle.radius) < circle.stroke)) shade = accent;
      if (x >= 12 && x < 116 && y >= 198 && y < 244) shade = 0;
      if (x >= 20 && x < 104 && y >= 205 && y < 235) {
        const slot = Math.floor((x - 20) / 42), column = Math.floor(((x - 20) % 42) / 6), line = Math.floor((y - 205) / 6);
        if (column < 3 && digits[Number(label[slot])]![line]![column] === "1") shade = 255;
      }
      image.data.set([shade, shade, shade, 255], (y * 256 + x) * 4);
    }
    await Bun.write(resolve(directory, `${label}.png`), PNG.sync.write(image));
    imports.push(`import cover${row} from "./artwork/${label}.png";`);
  }
  await Bun.write(resolve(fixture, ".ink/artwork.ts"), imports.join("\n")
    + `\n\nexport const artwork = [${Array.from({ length: 32 }, (_, index) => `cover${index + 1}`).join(", ")}];\n`);
}

if (import.meta.main) await generateArtwork(resolve(import.meta.dir, "../fixtures/android-app/visual"));
