import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";

const directory = process.argv[2] ?? "release/npm";
for (const filename of (await readdir(directory)).filter((name) => name.endsWith(".tgz")).sort()) {
  const archive = join(directory, filename);
  const pkg = JSON.parse(
    execFileSync("tar", ["-xOf", archive, "package/package.json"], { encoding: "utf8" }),
  );
  const response = await fetch(`https://registry.npmjs.org/${pkg.name}/${pkg.version}`);
  if (response.ok) {
    const existing = await response.json();
    const integrity =
      "sha512-" +
      createHash("sha512")
        .update(await readFile(archive))
        .digest("base64");
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
