import { dirname, resolve } from "node:path";
import { copyFile, mkdir, realpath, symlink } from "node:fs/promises";

const arguments_ = Bun.argv.slice(2);
const profileIndex = arguments_.indexOf("--profile");
const profile = profileIndex < 0 ? "ink-dev" : arguments_[profileIndex + 1];
if (!["ink-dev", "release"].includes(profile))
  throw new Error("Expected --profile ink-dev or release");
if (profileIndex >= 0) arguments_.splice(profileIndex, 2);
const suites = arguments_.length ? arguments_ : ["runtime", "scene"];
if (suites.some((suite) => !["runtime", "scene", "performance"].includes(suite)))
  throw new Error("Expected runtime, scene or performance");
const root = resolve(import.meta.dir, "../..");
const fixtures = {
  runtime: ["bridge", "split-web"],
  scene: ["lists", "views", "input", "navigation", "lifecycle"],
  performance: ["bindings", "react", "list", "rows", "navigation", "conversation", "playback"],
};
for (const suite of suites) {
  for (const fixture of fixtures[suite as keyof typeof fixtures]) {
    const fixtureRoot = resolve(root, "test/fixtures", suite, fixture);
    const modules = resolve(fixtureRoot, "node_modules");
    await mkdir(modules, { recursive: true });
    const dependencies = [
      ["ink", resolve(root, "packages/ink")],
      ["react", dirname(Bun.resolveSync("react/package.json", resolve(root, "packages/ink")))],
    ];
    if (fixture === "split-web") {
      await mkdir(resolve(modules, "@ink"), { recursive: true });
      dependencies.push(["@ink/network", resolve(root, "packages/network")]);
    }
    for (const [name, target] of dependencies) {
      const link = resolve(modules, name);
      if (await Bun.file(resolve(link, "package.json")).exists()) {
        if ((await realpath(link)) !== (await realpath(target)))
          throw new Error(`Unexpected fixture dependency: ${link}`);
      } else await symlink(target, link);
    }
    const configuration = resolve(fixtureRoot, "ink.toml");
    for (const splitWeb of fixture === "split-web" ? ["0", "1"] : ["0"]) {
      const child = Bun.spawn(
        [
          "cargo",
          "run",
          "--profile",
          profile,
          "-p",
          "ink-compiler",
          "--",
          "compile",
          configuration,
        ],
        {
          cwd: root,
          stdout: "inherit",
          stderr: "inherit",
          env: { ...process.env, INK_SPLIT_WEB: splitWeb },
        },
      );
      const code = await child.exited;
      if (code) process.exit(code);
      if (fixture === "split-web" && splitWeb === "0") {
        const assets = resolve(fixtureRoot, ".ink/android/assets");
        await copyFile(resolve(assets, "app.js"), resolve(assets, "app-inline.js"));
      }
    }
  }
}
