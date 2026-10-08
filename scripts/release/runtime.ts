import {
  mkdir,
  readFile,
  writeFile,
  readdir,
  rename,
  rm,
  symlink,
  stat,
  realpath,
} from "node:fs/promises";
import { resolve, dirname, join } from "node:path";
import { homedir } from "node:os";
import { createInterface } from "node:readline/promises";

const sdkRoot = resolve(import.meta.dir, "../..");
const sdk = await Bun.file(join(sdkRoot, "sdk.json")).json();
const [action, ...args] = process.argv.slice(2);
const home =
  process.env.INK_HOME ??
  (dirname(sdkRoot).endsWith("/versions")
    ? dirname(dirname(sdkRoot))
    : join(process.env.XDG_DATA_HOME ?? join(homedir(), ".local/share"), "ink"));
const firstParty = (name) => name === "ink" || name.startsWith("@ink/");
const publicName = (name) => (name === "ink" ? "ink-framework" : `ink-framework-${name.slice(5)}`);
const spec = (name) => `npm:${publicName(name)}@${sdk.version}`;
const packages = new Map();
for (const name of await readdir(join(sdkRoot, "packages"))) {
  const file = Bun.file(join(sdkRoot, "packages", name, "package.json"));
  if (!(await file.exists())) continue;
  const pkg = await file.json();
  packages.set(pkg.name, pkg);
}
const colour =
  process.stdout.isTTY &&
  process.env.NO_COLOR === undefined &&
  process.env.CLICOLOR !== "0" &&
  process.env.TERM !== "dumb";
const mark = (ready) => {
  const symbol = ready ? "✓" : "✗";
  return colour ? `\x1b[${ready ? 32 : 31}m${symbol}\x1b[0m` : symbol;
};
function step(label, detail, last = false) {
  console.log(`${last ? "└──" : "├──"} ${mark(true)} ${label.padEnd(12)} ${detail}`);
}
function run(command, cwd = sdkRoot, quiet = false) {
  const result = Bun.spawnSync(command, {
    cwd,
    stdin: "inherit",
    stdout: quiet ? "pipe" : "inherit",
    stderr: quiet ? "pipe" : "inherit",
  });
  if (result.exitCode) {
    if (quiet) {
      for (const bytes of [result.stdout, result.stderr]) {
        const text = bytes.toString().trim();
        if (text) console.error(text);
      }
    }
    throw new Error(`${command[0]} exited with ${result.exitCode}`);
  }
}
function capture(command) {
  if (!Bun.which(command[0])) return null;
  const result = Bun.spawnSync(command, { stdout: "pipe", stderr: "pipe" });
  return result.exitCode === 0 ? result.stdout.toString().trim() : null;
}
async function exists(path) {
  return !!path && !!(await stat(path).catch(() => null));
}
async function confirm(message) {
  if (!process.stdin.isTTY) throw new Error(`${message} Run ink setup in an interactive terminal.`);
  const input = createInterface({ input: process.stdin, output: process.stdout });
  try {
    return /^y(es)?$/i.test((await input.question(`${message} [y/N] `)).trim());
  } finally {
    input.close();
  }
}
async function synchronise(root, additions = []) {
  root = await realpath(root);
  const lock = join(root, ".ink", "install.lock");
  await mkdir(dirname(lock), { recursive: true });
  try {
    await mkdir(lock);
  } catch {
    throw new Error(`Another Ink install is running. If it was interrupted, remove ${lock}.`);
  }
  try {
    const path = join(root, "package.json");
    const original = await readFile(path, "utf8");
    const pkg = JSON.parse(original);
    pkg.dependencies ??= {};
    for (const name of additions) {
      if (!packages.has(name) || !name.startsWith("@ink/"))
        throw new Error(`Unknown Ink module: ${name}`);
      pkg.dependencies[name] = spec(name);
    }
    const required = new Set(["ink"]);
    for (const section of ["dependencies", "devDependencies", "optionalDependencies"]) {
      for (const name of Object.keys(pkg[section] ?? {})) {
        if (firstParty(name)) {
          required.add(name);
          pkg[section][name] = spec(name);
        }
      }
    }
    for (const name of required) {
      const definition = packages.get(name);
      if (!definition)
        throw new Error(
          `Ink ${sdk.version} no longer provides ${name}; remove or replace it in package.json.`,
        );
      for (const section of ["dependencies", "peerDependencies"]) {
        for (const dependency of Object.keys(definition[section] ?? {})) {
          if (firstParty(dependency)) required.add(dependency);
        }
      }
    }
    for (const name of required) pkg.dependencies[name] = spec(name);
    pkg.dependencies.react = sdk.react;
    for (const name of Object.keys(pkg.overrides ?? {})) {
      if (firstParty(name)) delete pkg.overrides[name];
    }
    if (pkg.overrides && !Object.keys(pkg.overrides).length) delete pkg.overrides;
    const next = JSON.stringify(pkg, null, 2) + "\n";
    const stamp = join(root, ".ink", "installed-version");
    const installed = await readFile(stamp, "utf8").catch(() => "");
    let packagesMatch = true;
    for (const name of required) {
      const actual = await Bun.file(join(root, "node_modules", name, "package.json"))
        .json()
        .catch(() => null);
      if (actual?.name !== publicName(name) || actual?.version !== sdk.version)
        packagesMatch = false;
    }
    if (next === original && installed === sdk.version && packagesMatch && action !== "install") {
      if (action === "add") {
        console.log("Ink · add");
        step("Packages", "already installed", true);
      }
      return;
    }
    console.log(`Ink · ${action === "prepare" ? "install" : action}`);
    const lockPath = join(root, "bun.lock");
    const oldLock = await readFile(lockPath).catch(() => null);
    await writeFile(path, next);
    try {
      // Bun reads saved overrides before reconciling them with package.json.
      if (oldLock) {
        const saved = Bun.JSONC.parse(oldLock.toString());
        const overrides = Object.keys(saved.overrides ?? {}).filter(firstParty);
        if (overrides.length) {
          for (const name of overrides) delete saved.overrides[name];
          if (!Object.keys(saved.overrides).length) delete saved.overrides;
          await writeFile(lockPath, JSON.stringify(saved, null, 2) + "\n");
        }
      }
      // Fresh releases may be missing from Bun's cached registry manifests.
      const refresh = installed !== sdk.version || !packagesMatch;
      run([process.execPath, "install", ...(refresh ? ["--no-cache"] : [])], root, true);
      await writeFile(stamp, sdk.version);
      step("Packages", `Ink ${sdk.version}`, additions.length === 0);
      if (additions.length) step("Add", additions.join(", "), true);
    } catch (error) {
      await writeFile(path, original);
      if (oldLock) await writeFile(lockPath, oldLock);
      else await rm(lockPath, { force: true });
      await rm(stamp, { force: true });
      throw error;
    }
  } finally {
    await rm(lock, { recursive: true, force: true });
  }
}
async function prerequisites(install) {
  if (process.platform === "linux" && process.arch === "arm64")
    throw new Error(
      "Android builds on Linux ARM64 are not supported by Ink: Google's Linux SDK/NDK host tools require x64. Use Linux x64 or macOS Apple Silicon for ink dev and ink build. Package installation and ink check remain available here.",
    );
  let failures = 0;
  const tree = action !== "prerequisites";
  if (tree) console.log(`Ink · ${action}`);
  const report = (label, ready, detail, guidance) => {
    if (tree || !ready)
      console.log(`${tree ? "├── " : ""}${mark(ready)} ${label.padEnd(16)} ${detail}`);
    if (!ready) {
      console.log(`${tree ? "│   " : "    "}${guidance}`);
      failures++;
    }
  };
  let rust = capture(["rustup", "run", sdk.rust, "rustc", "--version"]);
  if (!rust && install && (await confirm(`Install Rust ${sdk.rust} through rustup?`))) {
    if (!Bun.which("rustup"))
      throw new Error(
        "Install rustup from https://rustup.rs, then run ink setup again. Ink does not change your default toolchain.",
      );
    run([
      "rustup",
      "toolchain",
      "install",
      sdk.rust,
      "--profile",
      "minimal",
      "--target",
      "aarch64-linux-android",
    ]);
    rust = capture(["rustup", "run", sdk.rust, "rustc", "--version"]);
  }
  report(
    "Rust",
    !!rust,
    rust ? rust.split(" ")[1] : "not found",
    `Run rustup toolchain install ${sdk.rust} --profile minimal --target aarch64-linux-android`,
  );
  let targets = capture(["rustup", "target", "list", "--installed", "--toolchain", sdk.rust]);
  if (
    rust &&
    !targets?.includes("aarch64-linux-android") &&
    install &&
    (await confirm("Install the Android Rust target?"))
  ) {
    run(["rustup", "target", "add", "--toolchain", sdk.rust, "aarch64-linux-android"]);
    targets = capture(["rustup", "target", "list", "--installed", "--toolchain", sdk.rust]);
  }
  report(
    "Android target",
    !!targets?.includes("aarch64-linux-android"),
    targets?.includes("aarch64-linux-android") ? "aarch64-linux-android" : "not installed",
    `Run rustup target add --toolchain ${sdk.rust} aarch64-linux-android`,
  );
  let ndkTool = capture(["cargo", "ndk", "--version"]);
  if (
    !ndkTool &&
    rust &&
    install &&
    (await confirm(`Install cargo-ndk ${sdk.cargoNdk} in your Cargo bin directory?`))
  ) {
    run(["cargo", "install", "cargo-ndk", "--version", sdk.cargoNdk, "--locked"]);
    ndkTool = capture(["cargo", "ndk", "--version"]);
  }
  report(
    "cargo-ndk",
    !!ndkTool,
    ndkTool ? ndkTool.replace(/^cargo-ndk\s+/, "") : "not found",
    `Run cargo +${sdk.rust} install cargo-ndk --version ${sdk.cargoNdk} --locked`,
  );
  const javaConfigured =
    !process.env.JAVA_HOME || (await exists(join(process.env.JAVA_HOME, "bin/java")));
  const javaResult =
    javaConfigured && Bun.which("java")
      ? Bun.spawnSync(["java", "-version"], { stdout: "pipe", stderr: "pipe" })
      : { exitCode: 1, stdout: "", stderr: "" };
  const javaText = javaResult.stderr.toString() + javaResult.stdout.toString();
  const javaVersion = Number(javaText.match(/version "(\d+)/)?.[1]);
  // The release's Gradle supports these JDKs; prefer an existing installation.
  report(
    "Java",
    javaConfigured && javaResult.exitCode === 0 && javaVersion >= sdk.java && javaVersion <= 24,
    javaVersion ? String(javaVersion) : "not found",
    `Install JDK ${sdk.java}–24 or Android Studio and set JAVA_HOME${process.env.JAVA_HOME ? ` (currently ${process.env.JAVA_HOME})` : ""}. Then run ink setup.`,
  );
  let android = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT;
  if (!android)
    android = join(
      homedir(),
      process.platform === "darwin" ? "Library/Android/sdk" : "Android/Sdk",
    );
  let manager = join(android, "cmdline-tools/latest/bin/sdkmanager");
  if (!(await exists(manager))) manager = Bun.which("sdkmanager");
  const needs = [
    ["platform-tools", join(android, "platform-tools/adb"), "Platform tools", "installed"],
    [
      `platforms;android-${sdk.androidPlatform}`,
      join(android, `platforms/android-${sdk.androidPlatform}/android.jar`),
      "Android SDK",
      String(sdk.androidPlatform),
    ],
    [
      `build-tools;${sdk.androidBuildTools}`,
      join(android, `build-tools/${sdk.androidBuildTools}/apksigner`),
      "Build tools",
      sdk.androidBuildTools,
    ],
    [
      `ndk;${sdk.androidNdk}`,
      join(android, `ndk/${sdk.androidNdk}/source.properties`),
      "Android NDK",
      sdk.androidNdk,
    ],
  ];
  const missing = [];
  for (const [name, file] of needs) if (!(await exists(file))) missing.push(name);
  if (missing.length && install) {
    if (!manager)
      console.log(
        "Install Android SDK command-line tools through Android Studio or https://developer.android.com/studio#command-tools, then run ink setup again.",
      );
    else if (
      await confirm(
        `Install ${missing.join(", ")} into ${android}? Existing versions will remain installed.`,
      )
    ) {
      run([manager, `--sdk_root=${android}`, "--licenses"]);
      run([manager, `--sdk_root=${android}`, ...missing]);
    }
  }
  for (const [name, file, label, version] of needs) {
    const ready = await exists(file);
    report(
      label,
      ready,
      ready ? version : "not installed",
      `Run sdkmanager --sdk_root="${android}" "${name}"`,
    );
  }
  if (failures) {
    if (tree)
      console.log(
        `└── ${mark(false)} ${failures} missing or incompatible prerequisite${failures === 1 ? "" : "s"}\n`,
      );
    throw new Error(
      "Build prerequisites are incomplete. Run ink setup or follow the instructions above.",
    );
  }
  if (tree) console.log(`└── ${mark(true)} Ready to build\n`);
}
async function github(path) {
  const response = await fetch(`https://api.github.com/repos/${sdk.repository}/${path}`, {
    signal: AbortSignal.timeout(5000),
    headers: { Accept: "application/vnd.github+json" },
  });
  if (!response.ok) throw new Error(`GitHub returned ${response.status}`);
  return response.json();
}
async function latest() {
  const releases = await github("releases?per_page=100");
  return releases.find(
    (r) =>
      !r.draft && r.assets.some((a) => a.name === `ink-${process.platform}-${process.arch}.tar.gz`),
  );
}
async function notice() {
  if (process.env.CI || !process.stdout.isTTY) return;
  const path = join(home, "update-check.json");
  const cached = await Bun.file(path)
    .json()
    .catch(() => null);
  if (cached?.version && Bun.semver.order(cached.version, sdk.version) > 0)
    console.log(`Ink ${cached.version} is available. Run ink update.`);
  if (!cached || Date.now() - cached.checked > 86_400_000) {
    const child = Bun.spawn([process.execPath, import.meta.path, "refresh-notice"], {
      stdin: "ignore",
      stdout: "ignore",
      stderr: "ignore",
    });
    child.unref();
  }
}
async function update(version) {
  if (sdk.distribution !== "release")
    throw new Error("Run ink update from an installed release, not a source checkout.");
  console.log("Ink · update");
  const release = version
    ? await github(`releases/tags/v${version.replace(/^v/, "")}`)
    : await latest();
  if (!release) throw new Error("No published Ink release supports this computer yet.");
  const next = release.tag_name.replace(/^v/, "");
  if (!/^\d+\.\d+\.\d+(?:-[\w.]+)?$/.test(next)) throw new Error("Invalid release version");
  if (next === sdk.version || (!version && Bun.semver.order(next, sdk.version) < 0)) {
    console.log(`└── ${mark(true)} Already up to date · Ink ${sdk.version}`);
    return;
  }
  step("Check", `${sdk.version} → ${next}`);
  const name = `ink-${process.platform}-${process.arch}.tar.gz`;
  const asset = release.assets.find((a) => a.name === name);
  const checksums = release.assets.find((a) => a.name === "SHA256SUMS");
  if (!asset || !checksums) throw new Error("Release is missing its archive or checksums");
  await mkdir(join(home, "versions"), { recursive: true });
  const staging = join(home, "versions", `.download-${process.pid}`);
  await mkdir(staging);
  try {
    const download = async (url) => {
      const r = await fetch(url, { signal: AbortSignal.timeout(300_000) });
      if (!r.ok) throw new Error(`Download returned ${r.status}`);
      return r;
    };

    const sums = await (await download(checksums.browser_download_url)).text();
    const expected = sums
      .split("\n")
      .find((line) => line.endsWith(`  ${name}`))
      ?.split(" ")[0];
    const bytes = await (await download(asset.browser_download_url)).arrayBuffer();
    step("Download", `${process.platform}-${process.arch}`);
    const actual = new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
    if (!expected || actual !== expected) throw new Error("Release checksum mismatch");
    step("Verify", "checksum passed");
    const archive = join(staging, "release.tar.gz");
    await writeFile(archive, new Uint8Array(bytes));
    const payload = join(staging, "payload");
    await mkdir(payload);
    run(["tar", "-xzf", archive, "-C", payload]);
    const metadata = await Bun.file(join(payload, "sdk.json")).json();
    if (metadata.version !== next) throw new Error("Release archive version mismatch");
    run([join(payload, "bin/ink"), "--version"], sdkRoot, true);
    const destination = join(home, "versions", next);
    if (await exists(destination))
      throw new Error(`Release directory already exists: ${destination}`);
    await rename(payload, destination);
    const link = join(home, `.current-${process.pid}`);
    await symlink(destination, link);
    await rename(link, join(home, "current"));
    step("Install", `Ink ${next}`, true);
    console.log("\nProjects update on their next Ink command.");
  } finally {
    await rm(staging, { recursive: true, force: true });
  }
}
try {
  if (action === "prepare" || action === "install") {
    await notice();
    await synchronise(args[0]);
  } else if (action === "add") await synchronise(args.at(-1), args.slice(0, -1));
  else if (action === "setup" || action === "doctor" || action === "prerequisites")
    await prerequisites(action === "setup");
  else if (action === "update") await update(args[0]);
  else if (action === "refresh-notice") {
    await mkdir(home, { recursive: true });
    await writeFile(join(home, "update-check.json"), JSON.stringify({ checked: Date.now() }));
    const release = await latest();
    await writeFile(
      join(home, "update-check.json"),
      JSON.stringify({ checked: Date.now(), version: release?.tag_name.replace(/^v/, "") }),
    );
  } else throw new Error(`Unknown release command: ${action}`);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
