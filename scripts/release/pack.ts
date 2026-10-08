import { cp, mkdir, readFile, rm, writeFile, chmod, readdir } from "node:fs/promises";
import { resolve, dirname } from "node:path";

const root = resolve(import.meta.dir, "../..");
const sdk = await Bun.file(`${root}/sdk.json`).json();
const target = `${process.platform}-${process.arch}`;
if (!["darwin-arm64", "linux-x64", "linux-arm64"].includes(target))
  throw new Error(`Unsupported release target: ${target}`);
const out = resolve(process.argv[2] ?? `${root}/target/distribution`);
const stage = `${out}/ink-${sdk.version}-${target}`;
await mkdir(out, { recursive: true });
await rm(stage, { recursive: true, force: true });
await mkdir(stage);
function run(args, cwd = root) {
  const r = Bun.spawnSync(args, { cwd, stdout: "pipe", stderr: "inherit" });
  if (r.exitCode) throw new Error(`${args[0]} failed (${r.exitCode})`);
  return r.stdout.toString().trim();
}
const files = run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"])
  .split("\0")
  .filter(Boolean);
for (const file of files) {
  if (
    !/^(crates\/|platform\/android\/|vendor\/|assets\/(fonts|icons)\/|packages\/|tools\/oxlint\/|scripts\/(release\/runtime\.ts|ink)$|Cargo\.(toml|lock)$|rust-toolchain\.toml$|sdk\.json$|oxlint\.config\.mjs$|LICENSE$)/.test(
      file,
    )
  )
    continue;
  await mkdir(dirname(`${stage}/${file}`), { recursive: true });
  await cp(`${root}/${file}`, `${stage}/${file}`);
}
// The runtime workspace has no dependency on repository test fixtures.
await writeFile(
  `${stage}/Cargo.toml`,
  (await readFile(`${stage}/Cargo.toml`, "utf8")).replace('  "test/native",\n', ""),
);
const workspace = await Bun.file(`${root}/package.json`).json();
await writeFile(
  `${stage}/package.json`,
  JSON.stringify({ private: true, dependencies: workspace.devDependencies }, null, 2),
);
sdk.distribution = "release";
await writeFile(`${stage}/sdk.json`, JSON.stringify(sdk, null, 2) + "\n");
await mkdir(`${stage}/bin`);
await cp(`${root}/target/release/ink`, `${stage}/bin/ink`);
await cp(process.execPath, `${stage}/bin/bun`);
if (run([`${stage}/bin/bun`, "--version"]) !== sdk.bun) throw new Error(`Pack with Bun ${sdk.bun}`);
const bunSource = `https://github.com/oven-sh/bun/tree/bun-v${sdk.bun}`;
const bunLicence = await fetch(
  `https://raw.githubusercontent.com/oven-sh/bun/bun-v${sdk.bun}/LICENSE.md`,
);
if (!bunLicence.ok) throw new Error(`Could not fetch Bun ${sdk.bun} licence notices`);
await mkdir(`${stage}/licences`);
await writeFile(
  `${stage}/licences/bun.md`,
  `Bundled Bun ${sdk.bun}: ${bunSource}\n\n${await bunLicence.text()}`,
);
await cp(`${root}/bun.lock`, `${stage}/bun.lock`);
run([`${stage}/bin/bun`, "install", "--ignore-scripts"], stage);
await chmod(`${stage}/bin/ink`, 0o755);
run([`${stage}/bin/ink`, "--version"]);
await rm(`${out}/npm`, { recursive: true, force: true });
await mkdir(`${out}/npm`, { recursive: true });
for (const name of await readdir(`${stage}/packages`)) {
  const path = `${stage}/packages/${name}/package.json`;
  if (!(await Bun.file(path).exists())) continue;
  const pkg = await Bun.file(path).json();
  pkg.name = name === "ink" ? "ink-framework" : `ink-framework-${name}`;
  pkg.version = sdk.version;
  delete pkg.private;
  pkg.license = "MIT";
  pkg.repository = {
    type: "git",
    url: `git+https://github.com/${sdk.repository}.git`,
    directory: `packages/${name}`,
  };
  pkg.files = ["src", "*.json", "*.ts"];
  for (const kind of ["dependencies", "peerDependencies"]) {
    for (const key of Object.keys(pkg[kind] ?? {})) {
      if (key === "ink" || key.startsWith("@ink/")) {
        pkg[kind][key] =
          `npm:${key === "ink" ? "ink-framework" : `ink-framework-${key.slice(5)}`}@${sdk.version}`;
      }
    }
  }
  // Preserve internal package names in the SDK catalogue; publish transformed copies.
  const publishDir = `${out}/publish/${name}`;
  await rm(publishDir, { recursive: true, force: true });
  await cp(`${stage}/packages/${name}`, publishDir, { recursive: true });
  await writeFile(`${publishDir}/package.json`, JSON.stringify(pkg, null, 2) + "\n");
  await cp(`${root}/LICENSE`, `${publishDir}/LICENSE`);
  run([`${stage}/bin/bun`, "pm", "pack", "--destination", `${out}/npm`], publishDir);
}
const asset = `ink-${target}.tar.gz`;
run(["tar", "-czf", `${out}/${asset}`, "-C", stage, "."]);
const hash = new Bun.CryptoHasher("sha256")
  .update(await Bun.file(`${out}/${asset}`).arrayBuffer())
  .digest("hex");
await writeFile(`${out}/SHA256SUMS`, `${hash}  ${asset}\n`);
console.log(`Release ${sdk.version}: ${out}/${asset}`);
