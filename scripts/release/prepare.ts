import { readdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "../..");
const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9]+(?:\.[a-zA-Z0-9]+)*)?$/.test(version ?? "")) {
  throw new Error("Usage: bun scripts/release/prepare.ts <version>");
}
const sdk = await Bun.file(`${root}/sdk.json`).json();
sdk.version = version;
await writeFile(`${root}/sdk.json`, JSON.stringify(sdk, null, 2) + "\n");
for (const name of await readdir(`${root}/packages`)) {
  const path = `${root}/packages/${name}/package.json`;
  if (!(await Bun.file(path).exists())) continue;
  const pkg = await Bun.file(path).json();
  pkg.version = version;
  for (const kind of ["dependencies", "peerDependencies", "devDependencies"]) {
    for (const key of Object.keys(pkg[kind] ?? {})) {
      if (key === "ink" || key.startsWith("@ink/")) pkg[kind][key] = version;
    }
  }
  await writeFile(path, JSON.stringify(pkg, null, 2) + "\n");
}
const cargo = `${root}/crates/ink-cli/Cargo.toml`;
await writeFile(
  cargo,
  (await readFile(cargo, "utf8")).replace(/^version = ".*"/m, `version = "${version}"`),
);
console.log(`Prepared Ink ${version}. Refresh bun.lock and Cargo.lock before committing.`);
