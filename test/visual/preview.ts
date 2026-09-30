import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { PNG } from "pngjs";
import { decode, sha256 } from "./pixels";
import { stateNames } from "./capture";

// Diagnostic encoding copies only. Original adb screenshots and candidate
// metadata remain untouched, and approval is still a separate human action.
const args = Bun.argv.slice(2), index = args.indexOf("--candidates");
if (index < 0 || !args[index + 1]) throw Error("Usage: bun test/visual/preview.ts --candidates <directory>");
const directory = resolve(args[index + 1]!);
const manifest = await Bun.file(resolve(directory, "manifest.json")).json();
if (manifest.version !== 1 || manifest.status !== "candidate" || !Array.isArray(manifest.states)) throw Error("Expected a candidate manifest");
const output = resolve(directory, "previews");
await mkdir(output, { recursive: true });
const files: { original: string; preview: string; originalPngSha256: string; previewPngSha256: string; rgbaSha256: string }[] = [];
for (const state of manifest.states as { name: string; file: string; pngSha256: string; rgbaSha256: string }[]) {
  if (!stateNames.includes(state.name) || state.file !== `${state.name}.png`) throw Error("Unexpected state file");
  const bytes = await Bun.file(resolve(directory, state.file)).bytes();
  const image = decode(bytes);
  if (sha256(bytes) !== state.pngSha256 || sha256(image.data) !== state.rgbaSha256) throw Error(`Candidate changed: ${state.file}`);
  const preview = PNG.sync.write(image, { colorType: 6 });
  const path = resolve(output, state.file);
  await writeFile(path, preview);
  const saved = await Bun.file(path).bytes();
  if (sha256(decode(saved).data) !== state.rgbaSha256) throw Error(`Diagnostic pixels changed: ${state.file}`);
  files.push({ original: resolve(directory, state.file), preview: path,
    originalPngSha256: state.pngSha256, previewPngSha256: sha256(saved), rgbaSha256: state.rgbaSha256 });
  console.log(`Pixel-identical review preview: ${path}`);
}
await writeFile(resolve(output, "manifest.json"), JSON.stringify({ version: 1, purpose: "Encoding-only inspection diagnostic; identical decoded RGBA pixels; not approved references", files }, null, 2) + "\n");
