import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { capture, fixtureHash, root, regions, stateNames } from "./capture";
import { compare, comparisonTolerance, decode, matches, sha256, type Region } from "./pixels";

const references = resolve(root, "test/visual/references");
const approvalFile = Bun.file(resolve(references, "approval.json"));
if (!await approvalFile.exists()) throw Error("Visual references are unapproved. Review the candidate PNGs and explicitly approve them before running comparisons.");
const approval = await approvalFile.json();
const manifestBytes = await Bun.file(resolve(references, "manifest.json")).bytes();
const expected = JSON.parse(new TextDecoder().decode(manifestBytes));
if (approval.version !== 1 || approval.approved !== true || typeof approval.reviewedBy !== "string" || !approval.reviewedBy.trim()
  || approval.candidateManifestSha256 !== sha256(manifestBytes) || approval.fixtureSha256 !== await fixtureHash()
  || expected.fixtureSha256 !== approval.fixtureSha256 || !Array.isArray(approval.files) || approval.files.length !== stateNames.length) {
  throw Error("Reference approval is invalid or no longer matches the fixture and manifest");
}
const names = stateNames;
if (expected.version !== 1 || expected.status !== "candidate" || !Array.isArray(expected.states)
  || expected.states.length !== names.length || new Set(expected.states.map((state: { name: string }) => state.name)).size !== names.length) {
  throw Error("Approved state set is invalid");
}
for (const name of names) {
  const file = approval.files.find((file: { file: string }) => file.file === `${name}.png`);
  const state = expected.states.find((state: { name: string }) => state.name === name);
  if (!file || !state || file.pngSha256 !== state.pngSha256
    || sha256(await Bun.file(resolve(references, file.file)).bytes()) !== file.pngSha256) throw Error(`Approved file changed: ${name}`);
  if (!Array.isArray(state.regions) || JSON.stringify(state.regions.map(({ name, x, y, width, height, meaning }: Region) => ({ name, x, y, width, height, meaning }))) !== JSON.stringify(regions[name])) {
    throw Error(`Pixel region definitions changed since approval: ${name}`);
  }
}
const args = Bun.argv.slice(2), serialIndex = args.indexOf("--serial");
if (serialIndex < 0 || !args[serialIndex + 1]) throw Error("Usage: bun test/visual/run.ts --serial <adb-device-serial>");
const output = resolve(root, ".test-output/visual/comparisons", new Date().toISOString().replaceAll(":", "-"));
const actual = await capture(args[serialIndex + 1]!, output);
if (JSON.stringify(actual.states.map(state => state.name)) !== JSON.stringify(names)) throw Error("Captured state set differs from the approved states");
if (actual.device.fingerprint !== expected.device.fingerprint) throw Error("Device build differs from the reviewed references");
const results = [];
for (const state of actual.states) {
  const reference = expected.states.find((expected: { name: string }) => expected.name === state.name);
  const current = decode(await Bun.file(resolve(output, state.file)).bytes());
  const approved = decode(await Bun.file(resolve(references, reference.file)).bytes());
  for (const region of reference.regions as Region[]) results.push({ state: state.name, ...compare(current, approved, region) });
}
await mkdir(output, { recursive: true });
await writeFile(resolve(output, "comparison.json"), JSON.stringify({ version: 1, referenceApproval: approval, tolerance: comparisonTolerance, results }, null, 2) + "\n");
const failures = results.filter(result => !matches(result));
console.log(`${results.length - failures.length} pass\n${failures.length} fail`);
if (failures.length) {
  console.error(JSON.stringify(failures, null, 2));
  throw Error(`${failures.length} reviewed pixel regions differ; saved evidence: ${output}`);
}
console.log(`${results.length} approved pixel regions match within the shading tolerance. Evidence: ${output}`);
