import { appendFile, readFile, writeFile } from "node:fs/promises";
import { execFileSync, spawnSync } from "node:child_process";

const config = Bun.TOML.parse(await readFile("ink.toml", "utf8"));
const versionPattern = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const output = async (values) => {
  const lines = Object.entries(values).map(([name, value]) => `${name}=${value}\n`);
  await appendFile(process.env.GITHUB_OUTPUT, lines.join(""));
};
const run = (args) => execFileSync(args[0], args.slice(1), { encoding: "utf8" }).trim();

switch (process.argv[2]) {
  case "prepare": {
    const version = process.env.RELEASE_VERSION;
    if (!versionPattern.test(version ?? ""))
      throw new Error("Use a semantic version such as 0.2.0 or 0.2.0-beta.1");
    if (Bun.semver.order(version, config.version) <= 0)
      throw new Error(`Choose a version newer than ${config.version}`);
    config.version = version;
    config.version_code += 1;
    if (!Number.isSafeInteger(config.version_code) || config.version_code > 2100000000)
      throw new Error("Invalid Android version_code");
    await writeFile("ink.toml", Bun.TOML.stringify(config) + "\n");
    break;
  }
  case "metadata": {
    if (!versionPattern.test(config.version) || !Number.isInteger(config.version_code))
      throw new Error("ink.toml needs a valid version and version_code");
    const previous = spawnSync("git", ["show", `${process.env.BEFORE}:ink.toml`], {
      encoding: "utf8",
    });
    const old = previous.status === 0 ? Bun.TOML.parse(previous.stdout) : null;
    const changed =
      process.env.EVENT === "workflow_dispatch" || !old || old.version !== config.version;
    if (
      changed &&
      old &&
      process.env.EVENT !== "workflow_dispatch" &&
      (Bun.semver.order(config.version, old.version) <= 0 ||
        config.version_code <= old.version_code)
    )
      throw new Error("A release must increase both version and version_code");
    const pkg = JSON.parse(await readFile("package.json", "utf8"));
    const ink = pkg.dependencies?.ink?.match(
      /^npm:ink-framework@(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/,
    )?.[1];
    if (!ink)
      throw new Error(
        "Pin a published Ink version first: run ink install and commit package.json and bun.lock",
      );
    const slug = config.name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "");
    if (!slug) throw new Error("App name must contain a letter or number");
    await output({ changed, version: config.version, ink, apk: `${slug}-${config.version}.apk` });
    break;
  }
  case "toolchain": {
    const sdk = JSON.parse(await readFile(`${process.env.INK_HOME}/current/sdk.json`, "utf8"));
    await output({
      rust: sdk.rust,
      java: sdk.java,
      ndk: sdk.androidNdk,
      platform: sdk.androidPlatform,
      build_tools: sdk.androidBuildTools,
      cargo_ndk: sdk.cargoNdk,
    });
    break;
  }
  case "signing": {
    config.signing = {
      keystore: `${process.env.RUNNER_TEMP}/ink-release.jks`,
      key_alias: process.env.INK_KEY_ALIAS,
    };
    await writeFile("ink.toml", Bun.TOML.stringify(config) + "\n");
    break;
  }
  case "tag": {
    const tag = `v${config.version}`;
    const ref = `refs/tags/${tag}`;
    const existing = run(["git", "ls-remote", "origin", ref]);
    if (existing) {
      run(["git", "fetch", "origin", `${ref}:${ref}`]);
      if (run(["git", "rev-parse", `${tag}^{commit}`]) !== process.env.GITHUB_SHA)
        throw new Error(`${tag} already belongs to a different commit; choose a new version`);
    } else {
      run(["git", "tag", tag, process.env.GITHUB_SHA]);
      run(["git", "push", "origin", ref]);
    }
    break;
  }
  default:
    throw new Error("Unknown release operation");
}
