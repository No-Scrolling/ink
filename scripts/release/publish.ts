import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";

const directory = process.argv[2] ?? "release/npm";
const published = [];
for (const filename of (await readdir(directory)).filter((name) => name.endsWith(".tgz")).sort()) {
  const archive = join(directory, filename);
  const pkg = JSON.parse(
    execFileSync("tar", ["-xOf", archive, "package/package.json"], { encoding: "utf8" }),
  );
  const integrity =
    "sha512-" +
    createHash("sha512")
      .update(await readFile(archive))
      .digest("base64");
  published.push({ name: pkg.name, version: pkg.version, integrity });
  const response = await fetch(`https://registry.npmjs.org/${pkg.name}/${pkg.version}`);
  if (response.ok) {
    const existing = await response.json();
    if (existing.dist.integrity !== integrity)
      throw new Error(
        `${pkg.name}@${pkg.version} already exists with different contents. Choose a new release version.`,
      );
    console.log(`${pkg.name}@${pkg.version} is already published with the same contents.`);
    continue;
  }
  if (response.status !== 404)
    throw new Error(`Registry returned ${response.status} for ${pkg.name}`);
  execFileSync("npm", ["publish", archive, "--access", "public", "--tag", "latest"], {
    stdio: "inherit",
  });
}

// npm can accept a publication before its install metadata becomes readable.
const deadline = Date.now() + 600_000;
let pending = published;
while (pending.length) {
  const ready = await Promise.all(
    pending.map(async (pkg) => {
      const results = await Promise.all(
        ["application/json", "application/vnd.npm.install-v1+json"].map(async (accept) => {
          const response = await fetch(`https://registry.npmjs.org/${pkg.name}`, {
            headers: { Accept: accept },
            signal: AbortSignal.timeout(10_000),
          });
          if (!response.ok) throw new Error(`Registry returned ${response.status} for ${pkg.name}`);
          const metadata = await response.json();
          const version = metadata.versions?.[pkg.version];
          if (!version) return false;
          if (version.dist?.integrity !== pkg.integrity)
            throw new Error(`Registry integrity mismatch for ${pkg.name}@${pkg.version}`);
          return true;
        }),
      );
      return results.every(Boolean);
    }),
  );
  pending = pending.filter((_, index) => !ready[index]);
  if (!pending.length) break;
  const names = pending.map((pkg) => `${pkg.name}@${pkg.version}`).join(", ");
  if (Date.now() >= deadline)
    throw new Error(`npm packages are not available yet: ${names}. Retry this job later.`);
  console.log(`Waiting for npm package availability: ${names}`);
  await new Promise((resolve) => setTimeout(resolve, 10_000));
}
console.log("All npm packages are available for installation.");
