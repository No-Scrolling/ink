import { mkdir, realpath, symlink } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const root = resolve(import.meta.dir, "../..");
const modules = resolve(root, "test/fixtures/android-app/contracts/node_modules");
await mkdir(resolve(modules, "@ink"), { recursive: true });
await mkdir(resolve(modules, "@types"), { recursive: true });
for (const [name, target] of [
  ["ink", resolve(root, "packages/ink")],
  ["@ink/store", resolve(root, "packages/store")],
  ["@ink/network", resolve(root, "packages/network")],
  ["@ink/files", resolve(root, "packages/files")],
  ["react", dirname(Bun.resolveSync("react/package.json", resolve(root, "packages/ink")))],
  ["@types/react", resolve(root, "node_modules/@types/react")],
]) {
  const link = resolve(modules, name);
  if (await Bun.file(resolve(link, "package.json")).exists()) {
    if (await realpath(link) !== await realpath(target)) throw new Error(`Unexpected fixture dependency: ${link}`);
  } else await symlink(target, link);
}
