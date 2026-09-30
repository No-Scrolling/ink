import { copyFile, mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fixtureHash, root, regions, stateNames } from "./capture";
import { decode, inspect, sha256 } from "./pixels";

// Run only after the user has reviewed the candidates and explicitly
// authorised these exact reference images. Capture never calls this script.
const args = Bun.argv.slice(2);
const candidateIndex = args.indexOf("--candidates"), reviewerIndex = args.indexOf("--reviewed-by");
if (!args.includes("--approve") || candidateIndex < 0 || reviewerIndex < 0 || !args[reviewerIndex + 1]?.trim() || !args[candidateIndex + 1]) {
  throw Error("After human review: bun test/visual/approve.ts --candidates <directory> --reviewed-by <name> --approve");
}
const candidates = resolve(args[candidateIndex + 1]!);
const references = resolve(root, "test/visual/references");
if (await Bun.file(resolve(references, "approval.json")).exists()) throw Error("Approved references already exist; archive them explicitly before replacing them");
const manifestBytes = await Bun.file(resolve(candidates, "manifest.json")).bytes();
const manifest = JSON.parse(new TextDecoder().decode(manifestBytes));
if (manifest.version !== 1 || manifest.status !== "candidate" || manifest.fixtureSha256 !== await fixtureHash()
  || !Array.isArray(manifest.states) || manifest.states.length !== stateNames.length
  || new Set(manifest.states.map((state: { name: string }) => state.name)).size !== stateNames.length) throw Error("Candidates do not match the current fixture");
const files: { file: string; pngSha256: string }[] = [];
for (const name of stateNames) {
  const state = manifest.states.find((state: { name: string }) => state.name === name);
  if (!state || state.file !== `${name}.png`) throw Error(`Missing candidate state ${name}`);
  const bytes = await Bun.file(resolve(candidates, state.file)).bytes();
  const image = decode(bytes);
  if (sha256(bytes) !== state.pngSha256 || sha256(image.data) !== state.rgbaSha256
    || image.width !== 1080 || image.height !== 1240
    || JSON.stringify(regions[name]!.map(region => inspect(image, region))) !== JSON.stringify(state.regions)) {
    throw Error(`Candidate file or reviewed regions changed: ${state.file}`);
  }
  files.push({ file: state.file, pngSha256: state.pngSha256 });
}
await mkdir(references, { recursive: true });
for (const file of files) await copyFile(resolve(candidates, file.file), resolve(references, file.file));
await writeFile(resolve(references, "manifest.json"), manifestBytes);
await writeFile(resolve(references, "approval.json"), JSON.stringify({ version: 1, approved: true,
  reviewedBy: args[reviewerIndex + 1]!.trim(), approvedAt: new Date().toISOString(),
  candidateManifestSha256: sha256(manifestBytes), fixtureSha256: manifest.fixtureSha256, files }, null, 2) + "\n");
console.log(`Explicit review approval recorded in ${references}/approval.json`);
