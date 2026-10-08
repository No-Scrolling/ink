import { mkdir, readdir, readFile, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { decode, inspect, sha256, type Region } from "./pixels";
import { generateArtwork } from "./generate-artwork";

type CaptureState = { name: string; file: string; meaning: string; actions: string[]; pngSha256: string; rgbaSha256: string;
  stableCaptures: number; regions: ReturnType<typeof inspect>[] };

export const root = resolve(import.meta.dir, "../..");
export const fixture = resolve(root, "test/fixtures/android-app/visual");
const packageName = "com.vandam.ink.visualcontracts";
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
function terminateGroup(pid: number) {
  try { process.kill(-pid, "SIGKILL"); }
  catch (error) {
    if (!(error instanceof Error && "code" in error && error.code === "ESRCH")) throw error;
  }
}
export const regions: Record<string, Region[]> = {
  graphics: [
    { name: "header", x: 0, y: 0, width: 1080, height: 130, meaning: "Standard centred Graphics title" },
    { name: "canvas", x: 90, y: 220, width: 775, height: 250, meaning: "Canvas outline and primitives clipped at its right/bottom edges; large CLIPPED text cut at the vertical grey guide" },
    { name: "text", x: 90, y: 500, width: 760, height: 115, meaning: "Accented glyphs, line wrapping and the two-line ellipsis" },
    { name: "images", x: 90, y: 650, width: 705, height: 330, meaning: "Identical checkerboard asset in contain and cover modes with different letterboxing/cropping" },
  ],
  "list-start": [
    { name: "header", x: 0, y: 0, width: 1080, height: 130, meaning: "Standard centred List title above the list" },
    { name: "rows", x: 0, y: 130, width: 1080, height: 1110, meaning: "Initial native list window, accented text, spacing and scrollbar" },
  ],
  "list-end": [
    { name: "header", x: 0, y: 0, width: 1080, height: 130, meaning: "Standard centred List title after scrolling" },
    { name: "rows", x: 0, y: 130, width: 1080, height: 1110, meaning: "Final native list window, clipping at the viewport edges and end scrollbar position" },
  ],
  "rows-start": [
    { name: "header", x: 0, y: 0, width: 1080, height: 130, meaning: "Standard centred Rows title" },
    { name: "rows", x: 0, y: 130, width: 1080, height: 1110, meaning: "Reverb-style initial album rows: unique generated artwork, one-line titles, subtitles and eight-unit gaps" },
  ],
  "rows-end": [
    { name: "header", x: 0, y: 0, width: 1080, height: 130, meaning: "Standard centred Rows title after scrolling" },
    { name: "rows", x: 0, y: 130, width: 1080, height: 1110, meaning: "Final album rows with the correct artwork, clipped viewport edges and end scrollbar position" },
  ],
  "rows-return": [
    { name: "header", x: 0, y: 0, width: 1080, height: 130, meaning: "Standard centred Rows title after returning to the top" },
    { name: "rows", x: 0, y: 130, width: 1080, height: 1110, meaning: "Initial album rows restored after recycling: artwork and text remain paired correctly" },
  ],
};
export const stateNames = Object.keys(regions);

export async function fixtureHash() {
  const files = ["ink.toml", "package.json", "tsconfig.json", "app/index.tsx", "assets/pattern.png", ".ink/artwork.ts",
    ...(await readdir(resolve(fixture, ".ink/artwork"))).sort().map(name => `.ink/artwork/${name}`)];
  const hashInput: Uint8Array[] = [new TextEncoder().encode("generate-artwork.ts\0"),
    await Bun.file(resolve(import.meta.dir, "generate-artwork.ts")).bytes()];
  for (const file of files) hashInput.push(new TextEncoder().encode(file + "\0"), await Bun.file(resolve(fixture, file)).bytes());
  return sha256(Buffer.concat(hashInput));
}

export async function capture(serial: string, output: string) {
  if (!/^[\w.:-]+$/.test(serial)) throw Error("Invalid adb serial");
  if (await Bun.file(resolve(output, "manifest.json")).exists()) throw Error(`Refusing to overwrite captured candidates: ${output}`);
  await mkdir(output, { recursive: true });
  const logs = resolve(root, ".test-output/visual/logs", new Date().toISOString().replaceAll(":", "-"));
  await mkdir(logs, { recursive: true });
  let javaHome = process.env.JAVA_HOME;
  if (!javaHome && process.platform === "darwin") {
    const candidate = Bun.spawnSync(["/usr/libexec/java_home", "-v", "17"], { stdout: "pipe", stderr: "pipe" });
    if (candidate.exitCode === 0) javaHome = candidate.stdout.toString().trim();
  }
  if (!javaHome) throw Error("Set JAVA_HOME to JDK 17 or 21 before building the visual fixture");
  const javaVersion = Bun.spawnSync([resolve(javaHome, "bin/java"), "-version"], { stdout: "pipe", stderr: "pipe" });
  if (javaVersion.exitCode !== 0 || !/version "(17|21)\./.test(javaVersion.stdout.toString() + javaVersion.stderr.toString())) {
    throw Error("Visual capture requires JDK 17 or 21; Gradle does not support JDK 25");
  }
  async function command(argv: string[], label: string, timeoutMs = 30_000): Promise<string> {
    const child = Bun.spawn(argv, { cwd: root, env: { ...process.env, INK_SDK_ROOT: root, JAVA_HOME: javaHome }, detached: true, stdout: "pipe", stderr: "pipe" });
    const timeout = setTimeout(() => terminateGroup(child.pid), timeoutMs);
    try {
      const [stdout, stderr, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
      await writeFile(resolve(logs, `${label}.log`), stdout + stderr);
      if (code !== 0) throw Error(`${label} failed (${code}): ${stdout}${stderr}`);
      return stdout;
    } finally { clearTimeout(timeout); }
  }
  const adb = (args: string[], label: string) => command(["adb", "-s", serial, ...args], label);
  const lock = resolve(root, ".test-output", `android-${serial}.lock`);
  let locked = false, installed = false;
  try {
    await mkdir(lock); locked = true;
    await writeFile(resolve(lock, "owner.json"), JSON.stringify({ pid: process.pid, fixture, startedAt: new Date().toISOString() }));
    const avd = (await adb(["emu", "avd", "name"], "avd")).split(/\r?\n/)[0];
    if (avd !== "Light_Phone_III") throw Error("Visual capture requires the Light_Phone_III AVD");
    if ((await adb(["shell", "getprop", "sys.boot_completed"], "boot")).trim() !== "1") throw Error("Emulator is not booted");
    const size = await adb(["shell", "wm", "size"], "size"), density = await adb(["shell", "wm", "density"], "density");
    if (size.trim() !== "Physical size: 1080x1240" || density.trim() !== "Physical density: 480") throw Error("Visual capture requires unmodified 1080x1240 at 480 dpi");
    if ((await adb(["shell", "pm", "list", "packages", packageName], "packages")).split(/\r?\n/).includes(`package:${packageName}`)) {
      throw Error(`Fixture package already exists; refusing to replace it: ${packageName}`);
    }
    const modules = resolve(fixture, "node_modules");
    await mkdir(modules, { recursive: true });
    for (const [name, target] of [["ink", resolve(root, "packages/ink")], ["react", dirname(Bun.resolveSync("react/package.json", resolve(root, "packages/ink")))]] as const) {
      const link = resolve(modules, name);
      if (await Bun.file(resolve(link, "package.json")).exists()) {
        if (await realpath(link) !== await realpath(target)) throw Error(`Unexpected dependency: ${link}`);
      } else await symlink(target, link);
    }
    console.log("Building visual fixture through the production Ink CLI…");
    await generateArtwork(fixture);
    const fixtureSha256 = await fixtureHash();
    await command(["cargo", "run", "--profile", "ink-dev", "-p", "ink-cli", "--", "-C", fixture, "build", "--debug"], "build", 900_000);
    const apks = (await readdir(resolve(fixture, "dist"))).filter(name => name.endsWith("-debug.apk"));
    if (apks.length !== 1) throw Error("Expected exactly one freshly built fixture APK");
    const apk = resolve(fixture, "dist", apks[0]!);
    const apkSha256 = sha256(await Bun.file(apk).bytes());
    await adb(["install", apk], "install"); installed = true;
    await adb(["shell", "am", "start", "-W", "-n", `${packageName}/com.vandam.ink.MainActivity`], "launch");
    const pid = (await adb(["shell", "pidof", packageName], "pid")).trim();
    if (!/^\d+$/.test(pid)) throw Error("Fixture has no unique process");
    async function waitState(state: "graphics" | "list" | "rows") {
      const deadline = Date.now() + 20_000;
      while (Date.now() < deadline) {
        const log = await adb(["logcat", "-d", `--pid=${pid}`, "-v", "raw"], `state-${state}`);
        if (log.includes("JavaScript app failed:") || log.includes("FATAL EXCEPTION") || log.includes("Bundle activation failed")) throw Error(`Fixture runtime failed; see ${logs}`);
        if (log.includes(`INK_VISUAL_STATE {"version":1,"state":"${state}"}`)) return;
        await delay(250);
      }
      throw Error(`Fixture did not reach ${state}; see ${logs}`);
    }
    const states: CaptureState[] = [];
    async function screenshot(name: string, meaning: string, actions: string[]) {
      const file = `${name}.png`;
      let previousHash = "", stableCaptures = 0;
      const deadline = Date.now() + 20_000;
      while (Date.now() < deadline) {
        const child = Bun.spawn(["adb", "-s", serial, "exec-out", "screencap", "-p"], { detached: true, stdout: "pipe", stderr: "pipe" });
        const timeout = setTimeout(() => terminateGroup(child.pid), 10_000);
        let bytes: Uint8Array;
        try {
          const [data, stderr, code] = await Promise.all([new Response(child.stdout).bytes(), new Response(child.stderr).text(), child.exited]);
          if (code !== 0) throw Error(`Screenshot failed: ${stderr}`);
          bytes = data;
        } finally { clearTimeout(timeout); }
        await writeFile(resolve(output, file), bytes);
        // Inspect saved bytes rather than a potentially rescaled UI preview.
        const saved = await readFile(resolve(output, file));
        const image = decode(saved);
        if (image.width !== 1080 || image.height !== 1240) throw Error("Unexpected screenshot geometry");
        const hash = sha256(image.data);
        stableCaptures = hash === previousHash ? stableCaptures + 1 : 1;
        previousHash = hash;
        if (stableCaptures >= 3) {
          const inspected = regions[name]!.map(region => inspect(image, region));
          if (inspected.some(region => region.distinctColours < 5)) throw Error(`Captured region is blank: ${name}`);
          if (states.at(-1)?.rgbaSha256 === hash) throw Error(`State did not change: ${name}`);
          states.push({ name, file, meaning, actions, stableCaptures, pngSha256: sha256(saved), rgbaSha256: hash, regions: inspected });
          console.log(`Captured stable saved pixels: ${resolve(output, file)}`);
          return;
        }
        await delay(400);
      }
      throw Error(`Saved pixels did not stabilise for ${name}; candidate is not ready`);
    }
    await waitState("graphics");
    await screenshot("graphics", "Canvas and text clipping, accented line wrapping, image contain/cover", ["Fresh process launch; graphics state marker; three consecutive identical decoded-RGBA captures"]);
    await adb(["shell", "am", "start", "-W", "--activity-single-top", "-n", `${packageName}/com.vandam.ink.MainActivity`,
      "--es", "com.vandam.ink.notification.HREF", "/", "--es", "com.vandam.ink.notification.PARAMS", "'{\"fixtureState\":\"list\"}'"], "open-list");
    await waitState("list");
    await screenshot("list-start", "Initial compiled native list viewport", ["Open root route through the activity's normal notification intent with fixtureState=list; list state marker; three consecutive identical decoded-RGBA captures"]);
    for (let index = 0; index < 8; index++) {
      await adb(["shell", "input", "swipe", "550", "1100", "550", "450", "1000"], `scroll-${index}`);
      await delay(300);
    }
    await screenshot("list-end", "Compiled native list after eight upward swipes to its end", ["8 swipes (550,1100) to (550,450), each 1000 ms; 300 ms between swipes; three consecutive identical decoded-RGBA captures"]);
    await adb(["shell", "am", "start", "-W", "--activity-single-top", "-n", `${packageName}/com.vandam.ink.MainActivity`,
      "--es", "com.vandam.ink.notification.HREF", "/", "--es", "com.vandam.ink.notification.PARAMS", "'{\"fixtureState\":\"rows\"}'"], "open-rows");
    await waitState("rows");
    await screenshot("rows-start", "Initial Reverb-style album rows with deterministic generated artwork", ["Open root route with fixtureState=rows; rows state marker; three consecutive identical decoded-RGBA captures"]);
    for (let index = 0; index < 16; index++) {
      await adb(["shell", "input", "swipe", "550", "1100", "550", "450", "1000"], `rows-down-${index}`);
      await delay(300);
    }
    await screenshot("rows-end", "Album rows at the end of the list", ["16 upward swipes, each 1000 ms with 300 ms between; three consecutive identical decoded-RGBA captures"]);
    for (let index = 0; index < 16; index++) {
      await adb(["shell", "input", "swipe", "550", "450", "550", "1100", "1000"], `rows-up-${index}`);
      await delay(300);
    }
    await screenshot("rows-return", "Initial album rows after scrolling away and back", ["16 downward swipes, each 1000 ms with 300 ms between; three consecutive identical decoded-RGBA captures"]);
    if (await fixtureHash() !== fixtureSha256) throw Error("Fixture changed during capture; these images cannot be approved");
    const manifest = { version: 1, status: "candidate", capturedAt: new Date().toISOString(), fixture, fixtureSha256,
      apkSha256, renderer: "production Android Vulkan", device: { serial, avd, width: 1080, height: 1240, density: 480,
        fingerprint: (await adb(["shell", "getprop", "ro.build.fingerprint"], "fingerprint")).trim() },
      readiness: "App state markers; three consecutive identical decoded-RGBA captures; non-blank pixels; distinct state hashes; candidates require human review", states };
    await writeFile(resolve(output, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
    return manifest;
  } finally {
    try {
      if (installed) await adb(["uninstall", packageName], "uninstall");
    } finally {
      if (locked) await rm(lock, { recursive: true });
    }
  }
}

if (import.meta.main) {
  const args = Bun.argv.slice(2);
  const serialIndex = args.indexOf("--serial"), outputIndex = args.indexOf("--output");
  if (serialIndex < 0 || !args[serialIndex + 1]) throw Error("Usage: bun test/visual/capture.ts --serial <serial> [--output <directory>]");
  const output = resolve(outputIndex < 0 ? resolve(root, ".test-output/visual/candidates", new Date().toISOString().replaceAll(":", "-")) : args[outputIndex + 1]!);
  await capture(args[serialIndex + 1]!, output);
  console.log(`Candidate manifest: ${output}/manifest.json. Approval remains pending.`);
}
