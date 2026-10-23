import { copyFile, mkdir, readFile, readdir, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { homedir } from "node:os";
import { parseArgs } from "node:util";
import { decodePng, imageCount, imageSize, prepareImages, sha256, verifyAnyArtwork, verifyArtwork } from "./device-images";
import { collectProcess, cpuIntervals, energyIntervals, type ProcessSample } from "./device-observations";

const { values } = parseArgs({ options: {
  phase: { type: "string", default: "run" }, serial: { type: "string" }, output: { type: "string" },
  samples: { type: "string", default: "5" }, "duration-seconds": { type: "string", default: "120" },
  "idle-seconds": { type: "string", default: "30" }, "sample-interval-seconds": { type: "string", default: "5" },
  ink: { type: "string" }, help: { type: "boolean" },
} });
if (values.help) {
  console.log("Usage: bun test/performance/device-run.ts --phase build|run --serial SERIAL --output DIR [--samples 5] [--duration-seconds 120] [--idle-seconds 30] [--sample-interval-seconds 5] [--ink /absolute/path/to/ink]");
  console.log("Build creates a release APK and saved bundle without touching a device. Run uses only the saved bundle, requires a booted API 34+ device and cleans up its package/reverse mapping. No emulator is started or stopped.");
  process.exit(0);
}
const root = resolve(import.meta.dir, "../.."), fixture = resolve(root, "test/fixtures/performance/device");
const packageName = "com.vandam.ink.deviceperformance", activity = `${packageName}/com.vandam.ink.MainActivity`;
const output = resolve(values.output ?? resolve(root, ".test-output/performance-device", new Date().toISOString().replaceAll(":", "-") + `-${process.pid}`));
const bundle = resolve(output, "bundle"), logs = resolve(output, "logs"), devicePort = 18766;
const samples = Number(values.samples), durationSeconds = Number(values["duration-seconds"]);
const idleSeconds = Number(values["idle-seconds"]), intervalSeconds = Number(values["sample-interval-seconds"]);
if (!["build", "run"].includes(values.phase!)) throw Error("Phase must be build or run");
if (!Number.isInteger(samples) || samples < 1 || samples > 48) throw Error("Samples must be 1–48");
if (!Number.isInteger(durationSeconds) || durationSeconds < 120 || durationSeconds > 3600) throw Error("Sustained duration must be 120–3600 seconds");
if (!Number.isInteger(idleSeconds) || idleSeconds < 15 || idleSeconds > 600) throw Error("Idle duration must be 15–600 seconds");
if (!Number.isInteger(intervalSeconds) || intervalSeconds < 2 || intervalSeconds > 15) throw Error("Sample interval must be 2–15 seconds");
if (values.phase === "run" && (!values.serial || !/^[\w.:-]+$/.test(values.serial))) throw Error("Run requires an explicit --serial SERIAL");
if (values.phase === "run" && !values.output) throw Error("Run requires --output DIR from the completed build phase");
await mkdir(logs, { recursive: true });
const hashFile = async (file: string) => sha256(await readFile(file));
const json = (file: string, data: object) => writeFile(resolve(output, file), JSON.stringify(data, null, 2) + "\n");
const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;
let commandIndex = 0;
const active = new Set<ReturnType<typeof Bun.spawn>>();
let stopping = false;
function kill(child: ReturnType<typeof Bun.spawn>) {
  try { process.kill(-child.pid, "SIGKILL"); }
  catch (error) { if (!(error instanceof Error && "code" in error && error.code === "ESRCH")) throw error; }
}
for (const signal of ["SIGINT", "SIGTERM"] as const) process.on(signal, () => {
  stopping = true;
  for (const child of active) kill(child);
});

async function command(argv: string[], name: string, timeoutMs = 30_000, env = process.env, binary = false, cleanup = false) {
  if (stopping && !cleanup) throw Error("Benchmark interrupted; cleaning up");
  const child = Bun.spawn(argv, { cwd: root, env, detached: true, stdout: "pipe", stderr: "pipe" });
  active.add(child);
  const timeout = setTimeout(() => kill(child), timeoutMs);
  try {
    const [stdout, stderr, code] = await Promise.all([new Response(child.stdout).bytes(), new Response(child.stderr).text(), child.exited]);
    const prefix = `${String(++commandIndex).padStart(5, "0")}-${name.replaceAll(/[^\w.-]/g, "-")}`;
    await writeFile(resolve(logs, `${prefix}.log`), binary ? stderr : Buffer.from(stdout).toString() + stderr);
    if (code !== 0) throw Error(`${name} failed (${code}); see ${logs}/${prefix}.log: ${stderr.slice(-1500)}`);
    return stdout;
  } finally { clearTimeout(timeout); active.delete(child); }
}
const textCommand = async (argv: string[], name: string, timeoutMs = 30_000, env = process.env, cleanup = false) => Buffer.from(await command(argv, name, timeoutMs, env, false, cleanup)).toString();
const adb = (args: string[], name: string, cleanup = false) => textCommand(["adb", "-s", values.serial!, ...args], name, 30_000, process.env, cleanup);
const shell = (script: string, name: string) => adb(["shell", script], name);

async function sourceHashes() {
  const sources: Record<string, string> = {};
  async function visit(directory: string) {
    for (const entry of (await readdir(directory, { withFileTypes: true })).sort((left, right) => left.name.localeCompare(right.name))) {
      if ([".ink", "node_modules", "dist", "build", ".gradle"].includes(entry.name)) continue;
      const file = resolve(directory, entry.name);
      if (entry.isDirectory()) await visit(file);
      else if (entry.isFile()) sources[file.slice(root.length + 1)] = await hashFile(file);
    }
  }
  await visit(fixture);
  for (const file of ["device-run.ts", "device-images.ts", "device-observations.ts"]) sources[`test/performance/${file}`] = await hashFile(resolve(import.meta.dir, file));
  for (const file of ["platform/android/app/src/main/kotlin/com/vandam/ink/MainActivity.kt", "platform/android/native/src/android.rs", "platform/android/native/src/javascript.rs", "crates/ink-renderer-vulkan/src/compact.rs"]) sources[file] = await hashFile(resolve(root, file));
  return sources;
}

async function build() {
  if (await Bun.file(resolve(output, "build.json")).exists()) throw Error("Output already contains a build; use a new directory to retain the original APK");
  await mkdir(bundle, { recursive: true });
  const images = await prepareImages(fixture, resolve(bundle, "images"));
  const modules = resolve(fixture, "node_modules");
  await mkdir(resolve(modules, "@ink"), { recursive: true });
  for (const [name, target] of [
    ["ink", resolve(root, "packages/ink")], ["@ink/files", resolve(root, "packages/files")], ["@ink/network", resolve(root, "packages/network")],
    ["react", dirname(Bun.resolveSync("react/package.json", resolve(root, "packages/ink")))],
  ]) {
    const link = resolve(modules, name!);
    if (await Bun.file(resolve(link, "package.json")).exists()) {
      if (await realpath(link) !== await realpath(target!)) throw Error(`Unexpected dependency at ${link}`);
    } else await symlink(target!, link);
  }
  let javaHome = process.env.JAVA_HOME;
  if (!javaHome && process.platform === "darwin") javaHome = (await textCommand(["/usr/libexec/java_home", "-v", "17"], "java-home")).trim();
  const revision = (await textCommand(["git", "rev-parse", "HEAD"], "revision")).trim();
  // Public fixture-only signing key; never used for a distributed application.
  const password = "ink-benchmark", key = resolve(fixture, ".ink/benchmark.jks");
  if (!await Bun.file(key).exists()) await textCommand([javaHome ? resolve(javaHome, "bin/keytool") : "keytool", "-genkeypair", "-keystore", key, "-alias", "benchmark", "-keyalg", "RSA", "-keysize", "2048", "-validity", "3650", "-storepass", password, "-keypass", password, "-dname", "CN=Ink benchmark fixture", "-noprompt"], "fixture-key");
  const env = { ...process.env, JAVA_HOME: javaHome,
    ANDROID_HOME: process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT ?? resolve(homedir(), "Library/Android/sdk"),
    INK_SDK_ROOT: root, INK_BENCHMARK: "1", INK_BRIDGE_TIMING: "1", INK_MEMORY_DIAGNOSTICS: "1",
    INK_PRESENTATION_TIMING: "0", INK_BENCHMARK_REVISION: revision,
    INK_KEYSTORE_PASSWORD: password, INK_KEY_PASSWORD: password };
  const argv = values.ink ? [resolve(values.ink)] : ["cargo", "run", "--release", "-p", "ink-cli", "--"];
  console.log(`Building release Android benchmark; evidence: ${output}`);
  await textCommand([...argv, "-C", fixture, "build"], "build", 1_800_000, env);
  const apks = (await readdir(resolve(fixture, "dist"))).filter(file => file.endsWith(".apk") && !file.endsWith("-debug.apk"));
  if (apks.length !== 1) throw Error("Expected exactly one release fixture APK");
  await copyFile(resolve(fixture, "dist", apks[0]!), resolve(bundle, "device.apk"));
  for (const image of images) {
    const embedded = await command(["unzip", "-p", resolve(bundle, "device.apk"), `assets/ink-assets/${image.sha256}.png`], `verify-apk-image-${image.index}`, 30_000, process.env, true);
    if (embedded.length !== image.bytes || sha256(embedded) !== image.sha256) throw Error(`APK encoded asset differs: ${image.index}`);
  }
  await json("build.json", { version: 1, status: "built", createdAt: new Date().toISOString(), revision,
    apk: "bundle/device.apk", apkSha256: await hashFile(resolve(bundle, "device.apk")), images,
    sources: await sourceHashes(), dirty: (await textCommand(["git", "status", "--porcelain"], "source-status")).trim(),
    configuration: { apk: "release", rust: "release", javascript: "production QuickJS bytecode", features: ["benchmark", "bridge-timing", "memory-diagnostics"], presentationTiming: false, signing: "local public benchmark key" } });
  await writeFile(resolve(output, "tracked-changes.patch"), await textCommand(["git", "diff", "--binary", "HEAD"], "source-patch"));
  console.log(`Built ${resolve(bundle, "device.apk")}`);
}

type LogRecord = { pid: number; message: string; receivedHostMs: number };
type Frame = { scene: number; image_revision: number; scroll_y: number; ns: number; kind: string };
type Completion = { request_id: number; source: { kind: string; path?: string; encoded_bytes?: number }; width: number; height: number; decoded_bytes: number; started_ns: number; decoded_ns: number; accepted_ns: number; accepted: boolean; update: string; scene: number; image_revision: number; request: { source: string; module: string; ns: number } };
type Mark = { version: number; sequence: string; mode: string; index: number; epoch: string; source: string; ns: number; bootBeforeNs: number; bootAfterNs: number };
const records: LogRecord[] = [];
let recorder: ReturnType<typeof Bun.spawn> | undefined, recording: Promise<void> | undefined;
function startLogcat() {
  const child = Bun.spawn(["adb", "-s", values.serial!, "logcat", "-T", "1", "-v", "threadtime", "-s", "Ink:I", "InkResource:D", "AndroidRuntime:E"], { detached: true, stdout: "pipe", stderr: "pipe" });
  recorder = child;
  recording = (async () => {
    const file = Bun.file(resolve(output, "logcat.log")).writer(), decoder = new TextDecoder();
    let pending = "";
    const stderr = new Response(child.stderr).text();
    try {
      for await (const bytes of child.stdout) {
        file.write(bytes); pending += decoder.decode(bytes, { stream: true });
        let end: number;
        while ((end = pending.indexOf("\n")) >= 0) {
          const line = pending.slice(0, end); pending = pending.slice(end + 1);
          const match = line.match(/^\d\d-\d\d\s+\S+\s+(\d+)\s+\d+\s+[VDIWEF]\s+\S+\s*:\s?(.*)$/);
          if (match) records.push({ pid: Number(match[1]), message: match[2]!, receivedHostMs: performance.now() });
          if (records.length > 1_000_000) throw Error("Logcat evidence limit exceeded");
        }
      }
    } finally { await file.end(); await writeFile(resolve(logs, "logcat-stderr.log"), await stderr); }
  })();
  void recording.catch(() => {});
}
const frames = (events: LogRecord[]): Frame[] => events.flatMap(({ message }) => {
  return message.startsWith("InkImageFrame ") ? [{ ...JSON.parse(message.slice(14)), kind: "submitted" }] : [];
});
function completions(events: LogRecord[]): Completion[] {
  const requests = new Map<number, Completion["request"]>();
  return events.flatMap(({ message }) => {
    if (message.startsWith("InkImageRequest ")) { const request = JSON.parse(message.slice(16)); requests.set(request.request_id, request); }
    if (!message.startsWith("InkImageCompletion ")) return [];
    const completed = JSON.parse(message.slice(19)), request = requests.get(completed.request_id);
    if (!request) throw Error(`Native image completion has no request provenance: ${completed.request_id}`);
    return [{ ...completed, request }];
  });
}
const sourceUri = (completed: Completion) => completed.request.module === "assets" ? `asset://${completed.request.source}` : completed.request.source;
const memory = (events: LogRecord[]) => events.filter(event => event.message.startsWith("InkMemory ")).map(event => JSON.parse(event.message.slice(10)));
const perf = (events: LogRecord[]) => events.filter(event => event.message.startsWith("Perf ")).map(event => Object.fromEntries([...event.message.matchAll(/([a-z_]+)=(\S+)/g)].map(match => [match[1], /^\d+$/.test(match[2]!) ? Number(match[2]) : match[2]])));
const fixtureFiles = (pid: number) => records.filter(event => event.pid === pid && event.message.startsWith("InkDeviceFile ")).map(event => JSON.parse(event.message.slice(14)));
function checkErrors(pid: number) {
  const error = records.find(event => event.pid === pid && /INK_DEVICE_ERROR|JavaScript app failed|FATAL EXCEPTION|native panic|failed to render dirty frame/.test(event.message));
  if (error) throw Error(error.message);
  if (recorder?.exitCode !== null) throw Error("Logcat recorder exited before observation completed");
}
async function waitFor<T>(name: string, predicate: () => T | undefined, timeoutMs = 30_000): Promise<T> {
  const deadline = performance.now() + timeoutMs;
  while (performance.now() < deadline) {
    if (stopping) throw Error("Benchmark interrupted");
    const result = predicate(); if (result !== undefined) return result;
    await Bun.sleep(50);
  }
  throw Error(`Timed out waiting for ${name}`);
}
let sequenceNumber = 0;
const nextSequence = (prefix: string) => `${prefix}-${++sequenceNumber}`;
async function open(mode: string, index = 0, epoch = "shared", cold = false) {
  const sequence = nextSequence(mode), params = { mode, index, epoch, sequence };
  if (cold) await adb(["shell", "am", "force-stop", packageName], `force-stop-${sequence}`);
  const hostBeforeMs = performance.now();
  const launch = await adb(["shell", `am start -W --activity-single-top -n ${activity} --es com.vandam.ink.notification.HREF / --es com.vandam.ink.notification.PARAMS ${quote(JSON.stringify(params))}`], `open-${sequence}`);
  const hostAfterMs = performance.now();
  const pidText = (await adb(["shell", "pidof", packageName], `pid-${sequence}`)).trim();
  if (!/^\d+$/.test(pidText)) throw Error("Fixture does not have a unique process");
  const pid = Number(pidText);
  const mark = await waitFor(`native marker ${sequence}`, () => {
    checkErrors(pid);
    const event = records.find(event => event.pid === pid && event.message.startsWith("InkDeviceMark ") && event.message.includes(`"sequence":"${sequence}"`));
    return event ? JSON.parse(event.message.slice(14)) as Mark : undefined;
  });
  return { pid, mark, hostBeforeMs, hostAfterMs, launch };
}

async function savedScreenshot(name: string) {
  const bytes = await command(["adb", "-s", values.serial!, "exec-out", "screencap", "-p"], `screenshot-${name}`, 15_000, process.env, true);
  const file = resolve(output, `${name}.png`);
  await writeFile(file, bytes);
  const saved = await readFile(file), decoded = decodePng(saved);
  if (decoded.width !== 1080 || decoded.height !== 1240) throw Error("Device fixture requires the frozen 1080 × 1240 LP3 viewport; no device settings were changed");
  return { file: `${name}.png`, pngSha256: sha256(saved), rgbaSha256: sha256(decoded.data), decoded };
}

function inspectMultilingualPixels(image: ReturnType<typeof decodePng>) {
  const lines = [
    { script: "Greek", literal: "Ελληνικά · Καλημέρα κόσμε" },
    { script: "Cyrillic", literal: "Кириллица · Доброе утро" },
    { script: "Arabic", literal: "العربية · صباح الخير" },
    { script: "Chinese", literal: "中文 · 早上好世界" },
    { script: "Japanese", literal: "日本語 · おはよう世界" },
    { script: "Korean", literal: "한국어 · 좋은 아침" },
  ];
  const { width, height, data } = image;
  const histogram = new Uint32Array(256);
  for (let offset = 0; offset < data.length; offset += 4) histogram[data[offset]!] += 1;
  let background = 0;
  for (let value = 1; value < 256; value++) if (histogram[value]! > histogram[background]!) background = value;
  // Exclude the screen title; locate the six separated body lines from saved pixels.
  const left = Math.ceil(width * 0.035), right = width - left, top = Math.ceil(width * 0.13), bottom = height - Math.ceil(width * 0.035);
  const maximumGap = Math.ceil(width * 0.018), bands: { top: number; bottom: number; foregroundPixels: number }[] = [];
  let band: (typeof bands)[number] | undefined;
  for (let y = top; y < bottom; y++) {
    let foregroundPixels = 0;
    for (let x = left; x < right; x++) {
      const offset = (y * width + x) * 4;
      if (data[offset + 3] === 255 && Math.max(Math.abs(data[offset]! - background), Math.abs(data[offset + 1]! - background), Math.abs(data[offset + 2]! - background)) >= 24) foregroundPixels++;
    }
    if (foregroundPixels < 4) continue;
    if (!band || y - band.bottom > maximumGap) { band = { top: y, bottom: y, foregroundPixels: 0 }; bands.push(band); }
    band.bottom = y; band.foregroundPixels += foregroundPixels;
  }
  if (bands.length !== lines.length || bands.some(band => band.foregroundPixels < 100)) throw Error(`Saved multilingual screenshot has ${bands.length} non-blank line regions; expected six`);
  return { background, foregroundChannelDifference: 24, maximumGapPixels: maximumGap,
    boundary: "Pixel regions are associated with the six authored script lines in order. Non-blank pixels and region hashes do not establish glyph identity, complete script coverage or shaping correctness. Font cache counters describe the whole frame, not individual scripts.",
    lines: lines.map((line, index) => {
      const band = bands[index]!, region = { x: left, y: band.top, width: right - left, height: band.bottom - band.top + 1 };
      const bytes = new Uint8Array(region.width * region.height * 4);
      for (let row = 0; row < region.height; row++) bytes.set(data.subarray(((region.y + row) * width + region.x) * 4, ((region.y + row) * width + region.x + region.width) * 4), row * region.width * 4);
      return { ...line, region, foregroundPixels: band.foregroundPixels, rgbaSha256: sha256(bytes) };
    }) };
}

async function settle(opened: Awaited<ReturnType<typeof open>>, name: string, imageIndex?: number, multilingual = false) {
  const { pid, mark } = opened;
  await waitFor(`ready marker ${mark.sequence}`, () => {
    checkErrors(pid);
    return records.some(event => event.pid === pid && event.message.startsWith("INK_DEVICE_READY ") && event.message.includes(`"sequence":"${mark.sequence}"`)) ? true : undefined;
  });
  await waitFor(`submitted scene ${mark.sequence}`, () => frames(records.filter(event => event.pid === pid)).some(frame => frame.ns > mark.ns) ? true : undefined);
  let previousHash = "", previousFrameNs = 0;
  const deadline = performance.now() + 30_000;
  while (performance.now() < deadline) {
    checkErrors(pid);
    await Bun.sleep(400);
    const before = frames(records.filter(event => event.pid === pid)).at(-1)!;
    const capture = await savedScreenshot(name);
    await Bun.sleep(100);
    const events = records.filter(event => event.pid === pid), selectedFrames = frames(events).filter(frame => frame.ns >= mark.ns);
    const frame = selectedFrames.at(-1)!;
    if (frame.ns !== before.ns || capture.rgbaSha256 !== previousHash || frame.ns !== previousFrameNs) {
      previousHash = capture.rgbaSha256; previousFrameNs = frame.ns; continue;
    }
    const regions = imageIndex === undefined ? undefined : verifyArtwork(capture.decoded, imageIndex);
    const nativeCompletions = completions(events).filter(completed => completed.accepted && completed.accepted_ns <= frame.ns);
    const selectedSource = mark.mode === "gallery" ? `http://127.0.0.1:${devicePort}/images/${mark.epoch}/${imageIndex}.png` : mark.source;
    const imageCompletion = imageIndex === undefined ? undefined : nativeCompletions.findLast(completed => sourceUri(completed) === selectedSource && completed.image_revision <= frame.image_revision);
    if (imageIndex !== undefined && (!imageCompletion || imageCompletion.width !== 512 || imageCompletion.height !== 512 || imageCompletion.decoded_bytes !== 512 * 512 * 4)) throw Error(`No accepted full-size native image completion corresponds to the saved artwork: ${selectedSource}`);
    const diagnostics = memory(events).at(-1);
    if (!diagnostics) throw Error("APK lacks required memory-diagnostics renderer observations");
    if (imageIndex !== undefined && diagnostics.renderer.image_texture_bytes < imageSize * imageSize * 4) throw Error("Visible artwork has no matching decoded image texture allocation");
    let textPixels: ReturnType<typeof inspectMultilingualPixels> | undefined;
    let fontFallback: { characters: number; fallbackCharacters: number; missingCharacters: number; missingGlyphsObserved: boolean; boundary: string } | undefined;
    if (multilingual) {
      const { font_cache_characters: characters, font_cache_fallback_characters: fallbackCharacters, font_cache_missing_characters: missingCharacters } = diagnostics.renderer;
      if (![characters, fallbackCharacters, missingCharacters].every(value => Number.isInteger(value) && value >= 0)) throw Error("APK lacks ordinary font-cache character diagnostics required by the multilingual workload");
      if (fallbackCharacters < 1) throw Error("Multilingual frame did not exercise ordinary Android font fallback");
      fontFallback = { characters, fallbackCharacters, missingCharacters, missingGlyphsObserved: missingCharacters > 0,
        boundary: "Ordinary GlyphAtlas cached character/size entries: fallback uses font_index > 0; missing uses glyph id 0. Missing characters are retained without a zero-missing gate. These whole-frame counters do not establish per-script coverage or shaping support." };
      textPixels = inspectMultilingualPixels(capture.decoded);
    }
    return { ...opened, elapsedNs: frame.ns - mark.ns, endpoint: frame, submittedFrames: selectedFrames, imageCompletion,
      dimensions: imageIndex === undefined ? undefined : { encoded: { width: 512, height: 512 }, decoded: { width: imageCompletion!.width, height: imageCompletion!.height, bytes: imageCompletion!.decoded_bytes }, displayed: { width: regions!.width, height: regions!.height }, layout: { width: 512 / 2.55, height: 512 / 2.55, viewportScale: 2.55 } },
      screenshot: { file: capture.file, pngSha256: capture.pngSha256, rgbaSha256: capture.rgbaSha256, width: capture.decoded.width, height: capture.decoded.height, artworkPixels: regions, textPixels, stableCaptures: 2 },
      rendererMemory: diagnostics, fontFallback, rendererPerf: perf(events.slice(events.findIndex(event => event.message.startsWith("InkDeviceMark ") && event.message.includes(`"sequence":"${mark.sequence}"`)))) };
  }
  throw Error(`Screenshot and renderer frame did not settle for ${name}`);
}

async function run() {
  if (await Bun.file(resolve(output, "result.json")).exists()) throw Error("Output already has run evidence; retain it and choose another build directory");
  const built = await Bun.file(resolve(output, "build.json")).json();
  if (built.version !== 1 || built.status !== "built" || await hashFile(resolve(output, built.apk)) !== built.apkSha256) throw Error("Saved build is missing or APK hash differs");
  const replayFile = Bun.file(resolve(output, "replay.json"));
  const replay = await replayFile.exists() ? await replayFile.json() : undefined;
  const expectedSources = { ...built.sources };
  if (replay) {
    const runnerFiles = ["device-run.ts", "device-images.ts", "device-observations.ts"].map(file => `test/performance/${file}`);
    if (replay.buildManifestSha256 !== await hashFile(resolve(output, "build.json"))
      || Object.keys(replay.runnerSources).some(file => !runnerFiles.includes(file))) throw Error("Runner replay must reference this build and change only host runner sources");
    Object.assign(expectedSources, replay.runnerSources);
  }
  const current = await sourceHashes();
  if (JSON.stringify(current) !== JSON.stringify(expectedSources)) throw Error("Device workload sources differ from the saved build or recorded host-runner replay");
  for (const image of built.images) if (await hashFile(resolve(bundle, "images", image.file)) !== image.sha256) throw Error(`Encoded fixture hash differs: ${image.file}`);
  const lock = resolve(root, ".test-output", `android-${values.serial}.lock`);
  const result: { [key: string]: unknown; errors: string[] } = { version: 1, status: "running", startedAt: new Date().toISOString(), build: built, settings: { samples, durationSeconds, idleSeconds, intervalSeconds }, errors: [],
    ...(replay ? { runnerReplay: replay } : {}),
    boundaries: {
      imageLatency: "Fixture native monotonic mark before React view update to final settled Vulkan queue submission; request-ID/source provenance and accepted native decoder completion identify the image. Saved pixels verify that submitted scene contains it. Encoded, actual decoded and displayed dimensions are separate. Includes JS reply/React, source loading, Android AImageDecoder, upload and submission. Intentionally delayed loopback HTTP transport is included. Host response readiness is only transport evidence; no display presentation endpoint.",
      coldStart: "am force-stop retains filesystem/OS caches and managed files; process start ticks through first and final content submission. Fresh process/renderer/QuickJS state, not a reboot, package-data reset or storage-cold start. CLK_TCK resolution and paired CLOCK_BOOTTIME/CLOCK_MONOTONIC conversion uncertainty are retained.",
      multilingual: "Distinct fresh-process multilingual screen: whole-screen submission includes ordinary Android font fallback. A positive font_cache_fallback_characters count establishes ordinary fallback activity; actual font_cache_missing_characters is retained without requiring zero. Six separately checked saved line regions retain their literal scripts and hashes. Glyph preparation is not isolated; neither non-blank pixels nor whole-frame cache counts establish per-script coverage, glyph identity or shaping correctness. The emoji system-glyph atlas is not a gate.",
      memoryCpu: "Real /proc process CPU ticks, /proc/stat and dumpsys meminfo PSS; sample collection and adb input/logging overhead included. Renderer counters are separate from whole-process PSS. No forced GC or claimed leak proof.",
      energy: "Physical whole-device charge/energy counters only when readable. Emulator battery values are never collected or treated as hardware. No app-attributed energy, battery-life prediction or SurfaceView gfxinfo FrameTimeline presentation claim.",
    } };
  let locked = false, installed = false, reversed = false;
  let server: ReturnType<typeof Bun.serve> | undefined;
  const requests: { path: string; method: string; index?: number; receivedHostMs: number; respondedHostMs?: number; bytes?: number; sha256?: string; delayMs?: number; accept: string | null }[] = [];
  try {
    await mkdir(lock); locked = true;
    if ((await adb(["shell", "getprop", "sys.boot_completed"], "boot")).trim() !== "1") throw Error("Device has not completed boot");
    const packages = await adb(["shell", "pm", "list", "packages", packageName], "existing-package");
    if (packages.split(/\r?\n/).includes(`package:${packageName}`)) throw Error(`Refusing to replace existing fixture ${packageName}`);
    if ((await adb(["reverse", "--list"], "existing-reverse")).split("\n").some(line => line.trim().split(/\s+/)[1] === `tcp:${devicePort}`)) throw Error(`Device port ${devicePort} already has a reverse mapping`);
    const properties = await shell("getprop", "device-properties");
    const property = (name: string) => properties.split("\n").find(line => line.startsWith(`[${name}]: [`))?.split("]: [")[1]?.slice(0, -1) ?? "";
    const emulator = values.serial!.startsWith("emulator-") || property("ro.kernel.qemu") === "1" || property("ro.boot.qemu") === "1";
    const ticks = Number((await shell("getconf CLK_TCK", "clock-ticks")).trim());
    if (!Number.isInteger(ticks) || ticks <= 0) throw Error("Device does not expose its actual CLK_TCK");
    result.device = { serial: values.serial, emulator, hardware: property("ro.hardware"), model: property("ro.product.model"), fingerprint: property("ro.build.fingerprint"), sdk: Number(property("ro.build.version.sdk")), clockTicksPerSecond: ticks,
      display: await shell("wm size; wm density", "display"), kernel: await shell("uname -a", "kernel") };
    if (Number(property("ro.build.version.sdk")) < 34) throw Error("API 34+ is required for the production network/image loader");
    const bytes = await Promise.all(built.images.map((image: { file: string }) => readFile(resolve(bundle, "images", image.file))));
    server = Bun.serve({ hostname: "127.0.0.1", port: 0, async fetch(request) {
      const url = new URL(request.url), match = url.pathname.match(/^\/images\/([\w-]+)\/(\d+)\.png$/);
      const observation: (typeof requests)[number] = { path: url.pathname, method: request.method, receivedHostMs: performance.now(), accept: request.headers.get("accept") };
      requests.push(observation);
      if (request.method !== "GET" || !match || Number(match[2]) >= imageCount) return new Response("Unexpected benchmark request", { status: 404 });
      const index = Number(match[2]), delayMs = match[1]!.startsWith("gallery") ? 30 + index % 5 * 20 : 60;
      await Bun.sleep(delayMs);
      Object.assign(observation, { index, delayMs, respondedHostMs: performance.now(), bytes: bytes[index]!.length, sha256: sha256(bytes[index]!) });
      return new Response(bytes[index]!, { headers: { "content-type": "image/png", "cache-control": "no-store", "content-length": String(bytes[index]!.length) } });
    } });
    await adb(["reverse", `tcp:${devicePort}`, `tcp:${server.port}`], "reverse"); reversed = true;
    await adb(["install", resolve(output, built.apk)], "install"); installed = true;
    startLogcat();
    console.log(`Measuring ${values.serial}; sustained scrolling ${durationSeconds}s plus idle observations. Evidence: ${output}`);

    const cold: object[] = [], multilingual: object[] = [];
    result.coldStart = cold; result.multilingualColdStart = multilingual;
    for (const mode of ["asset", "multilingual"]) for (let sample = 0; sample < samples; sample++) {
      const opened = await open(mode, sample, `cold-${sample}`, true);
      const observation = await settle(opened, `cold-${mode}-${sample}`, mode === "asset" ? sample : undefined, mode === "multilingual");
      const stat = (await shell(`cat /proc/${opened.pid}/stat; cat /proc/uptime`, `start-stat-${mode}-${sample}`)).trim().split("\n");
      const startTicks = Number(stat[0]!.slice(stat[0]!.lastIndexOf(")") + 2).split(/\s+/)[19]);
      const first = frames(records.filter(record => record.pid === opened.pid))[0]!;
      const bootMinusMonotonicNs = (opened.mark.bootBeforeNs + opened.mark.bootAfterNs) / 2 - opened.mark.ns;
      const startNs = startTicks / ticks * 1e9 - bootMinusMonotonicNs;
      const measured = { ...observation, processStartTicks: startTicks, processStartTickResolutionNs: 1e9 / ticks,
        processStartToFirstSubmittedNs: first.ns - startNs, processStartToContentSubmittedNs: observation.endpoint.ns - startNs,
        bootMinusMonotonicNs, clockPairUncertaintyNs: (opened.mark.bootAfterNs - opened.mark.bootBeforeNs) / 2,
        clockBoundary: "Process start ticks use CLOCK_BOOTTIME; renderer uses CLOCK_MONOTONIC. Paired elapsedRealtimeNanos/nanoTime calls at the fixture marker supply the measured offset. Suspend during a sample invalidates this conversion; tick quantisation is additional." };
      if (measured.processStartToFirstSubmittedNs < 0 || measured.processStartToContentSubmittedNs < 0) throw Error("Device start and renderer clocks differ; refusing an invalid cold-start duration");
      (mode === "asset" ? cold : multilingual).push(measured);
    }
    const imageLoads: object[] = [];
    result.imageLoads = imageLoads;
    for (const mode of ["asset", "file", "network"]) for (let sample = 0; sample < samples; sample++) {
      await settle(await open("blank"), `blank-${mode}-${sample}`);
      const imageIndex = (sample + 1) % imageCount;
      const opened = await open(mode, imageIndex, `${mode}-${sample}`);
      const observation = await settle(opened, `image-${mode}-${sample}`, imageIndex);
      const file = fixtureFiles(opened.pid).find(file => file.index === imageIndex), expected = built.images[imageIndex];
      if (mode === "file" && (!file || file.sha256 !== expected.sha256 || file.bytes !== expected.bytes || file.width !== 512 || file.height !== 512 || file.file.size !== expected.bytes || file.validationDecodedWidth !== 512 || file.validationDecodedHeight !== 512)) throw Error(`Managed file provenance differs for ${sample}`);
      const expectedSource = mode === "asset" ? `asset://ink-assets/${expected.sha256}.png` : mode === "file" ? file.file.src : `http://127.0.0.1:${devicePort}/images/${mode}-${sample}/${imageIndex}.png`;
      if (opened.mark.source !== expectedSource) throw Error("Native fixture marker selected a different source URI");
      if (mode === "network" && !requests.some(request => request.path === `/images/${mode}-${sample}/${imageIndex}.png` && request.sha256 === expected.sha256 && request.bytes === expected.bytes && request.accept?.includes("image/"))) throw Error("Selected image did not pass through the Android network image request");
      imageLoads.push({ ...observation, source: file, encoded: expected,
        cacheBoundary: mode === "network" ? "Unique URL and no-store HTTP response; native image cache miss" : "Distinct source per mode/sample in this process; OS filesystem/page caches may be warm",
        decodedSizeBoundary: "Encoded source dimensions, actual native AImageDecoder completion dimensions/bytes and independently checked physical display dimensions are reported separately. Frozen viewport 1080×1240; 512/2.55 layout units yield the 512-pixel authored oracle." });
    }
    const resume: object[] = [];
    result.lifecycle = resume;
    for (let sample = 0; sample < samples; sample++) {
      await adb(["shell", "input", "keyevent", "KEYCODE_HOME"], `background-${sample}`);
      await Bun.sleep(500);
      const before = records.length;
      const opened = await open("network", 1, "network-0");
      const observation = await settle(opened, `resume-${sample}`, 1);
      resume.push({ ...observation, firstSubmittedAfterCommand: frames(records.slice(before).filter(record => record.pid === opened.pid))[0],
        boundary: "Host am-start dispatch/return and native fixture mark-to-content submission after backgrounding. Native onResume entry is not instrumented; no claimed pure onResume latency." });
    }
    const cacheProof: Awaited<ReturnType<typeof settle>>[] = [];
    result.cacheTurnoverProof = cacheProof;
    for (let index = 0; index < 16; index++) cacheProof.push(await settle(await open("network", index, "cache-proof"), `cache-proof-${index}`, index));
    const revisitProof = await settle(await open("network", 0, "cache-proof"), "cache-proof-revisit-0", 0);
    if (!revisitProof.imageCompletion || revisitProof.imageCompletion.accepted_ns <= cacheProof[15]!.endpoint.ns) throw Error("Cache revisit did not trigger a fresh accepted native decode after the >8 MiB traversal");
    const revisitUploadedBytes = revisitProof.rendererPerf.reduce((sum, metrics) => sum + Number(metrics.uploaded_bytes ?? 0), 0);
    const revisitCacheMisses = revisitProof.rendererPerf.reduce((sum, metrics) => sum + Number(metrics.cache_misses ?? 0), 0);
    if (revisitUploadedBytes < revisitProof.imageCompletion.decoded_bytes || revisitCacheMisses < 1) throw Error("Cache revisit has no matching renderer upload/cache-miss evidence");
    cacheProof.push(revisitProof);
    result.cacheTurnoverBoundary = { distinctPixelVerifiedArtworks: 16, measuredDecodedWorkingSetBytes: cacheProof.slice(0, 16).reduce((sum, observation) => sum + observation.imageCompletion!.decoded_bytes, 0),
      retentionTargetBytes: 8 * 1024 * 1024, revisitCompletion: revisitProof.imageCompletion, revisitUploadedBytes, revisitCacheMisses,
      boundary: "Controlled sequential network traversal: every distinct artwork has request provenance, accepted native decoded dimensions/bytes, a corresponding submitted image revision and a saved pixel-verified identity. The revisit requires a new accepted decode of the original URI after 16 MiB. This is separate from sustained swipe observations; server response readiness is not counted as decoded work." };
    const gallery = await open("gallery", 0, "gallery-sustained");
    await settle(gallery, "gallery-start", 0);
    const processSamples: ProcessSample[] = [];
    const scrollingPixels: { file: string; pngSha256: string; rgbaSha256: string; artwork: ReturnType<typeof verifyAnyArtwork>; submittedBeforeCapture: Frame; submittedAfterCapture: Frame; captureHostBeforeMs: number; captureHostAfterMs: number; stableDuringObservation: boolean; imageCompletion: Completion }[] = [];
    const skippedCaptures: { file: string; reason: string }[] = [];
    const swipes: { index: number; direction: string; hostBeforeMs: number; hostAfterMs: number }[] = [];
    result.processSamples = processSamples;
    result.sustainedProgress = { swipes, scrollingPixels, skippedCaptures };
    const sampleProcess = async (phase: string) => { const value = await collectProcess(shell, packageName, phase, ticks, emulator); processSamples.push(value); return value; };
    await sampleProcess("before-idle");
    const observeIdle = async (prefix: string, seconds: number) => {
      const began = performance.now(), startRecord = records.length;
      let nextSample = began + intervalSeconds * 1000;
      while (performance.now() - began < seconds * 1000) {
        if (stopping) throw Error("Benchmark interrupted");
        await Bun.sleep(Math.min(250, Math.max(1, nextSample - performance.now())));
        if (performance.now() >= nextSample) { await sampleProcess(`${prefix}-${processSamples.length}`); nextSample += intervalSeconds * 1000; }
      }
      return { requestedSeconds: seconds, elapsedSeconds: (performance.now() - began) / 1000, submittedFrames: frames(records.slice(startRecord).filter(record => record.pid === gallery.pid)), rendererPerf: perf(records.slice(startRecord).filter(record => record.pid === gallery.pid)) };
    };
    result.preWorkIdle = await observeIdle("idle-before", Math.min(15, idleSeconds));
    await sampleProcess("before-sustained");
    const began = performance.now(); let nextSample = began, swipe = 0;
    let galleryView = (await savedScreenshot("gallery-geometry")).decoded;
    while (performance.now() - began < durationSeconds * 1000) {
      checkErrors(gallery.pid);
      const up = Math.floor(swipe / 48) % 2 === 0;
      const x = Math.floor(galleryView.width / 2), top = Math.floor(galleryView.height * 0.3), bottom = Math.floor(galleryView.height * 0.85);
      const at = performance.now();
      await adb(["shell", "input", "swipe", String(x), String(up ? bottom : top), String(x), String(up ? top : bottom), "300"], `sustained-swipe-${swipe}`);
      swipes.push({ index: swipe++, direction: up ? "forward" : "back", hostBeforeMs: at, hostAfterMs: performance.now() });
      if (performance.now() >= nextSample) { await sampleProcess(`sustained-${swipe}`); nextSample = performance.now() + intervalSeconds * 1000; }
      if (swipe % (new Set(scrollingPixels.map(capture => capture.artwork.index)).size < 12 ? 2 : 8) === 0) {
        await Bun.sleep(200);
        const before = frames(records.filter(record => record.pid === gallery.pid)).at(-1)!;
        const captureHostBeforeMs = performance.now();
        const capture = await savedScreenshot(`gallery-swipe-${swipe}`);
        const captureHostAfterMs = performance.now();
        galleryView = capture.decoded;
        await Bun.sleep(100);
        const after = frames(records.filter(record => record.pid === gallery.pid)).at(-1)!;
        let artwork: ReturnType<typeof verifyAnyArtwork>;
        try { artwork = verifyAnyArtwork(capture.decoded); }
        catch (error) {
          if (!(error instanceof Error) || error.message !== "Saved scrolling screenshot contains no complete, recognised artwork") throw error;
          skippedCaptures.push({ file: capture.file, reason: error.message });
          continue;
        }
        const imageCompletion = completions(records.filter(record => record.pid === gallery.pid)).findLast(completed => completed.accepted && sourceUri(completed) === `http://127.0.0.1:${devicePort}/images/gallery-sustained/${artwork.index}.png` && completed.accepted_ns <= before.ns && completed.image_revision <= before.image_revision);
        if (!imageCompletion) {
          skippedCaptures.push({ file: capture.file, reason: "No matching native completion and submission observed before capture" });
          continue;
        }
        if (imageCompletion.width !== imageSize || imageCompletion.height !== imageSize || imageCompletion.decoded_bytes !== imageSize * imageSize * 4) throw Error("Scrolling artwork was not decoded at the expected dimensions and byte size");
        scrollingPixels.push({ file: capture.file, pngSha256: capture.pngSha256, rgbaSha256: capture.rgbaSha256, artwork, submittedBeforeCapture: before, submittedAfterCapture: after, captureHostBeforeMs, captureHostAfterMs, stableDuringObservation: before.ns === after.ns, imageCompletion });
      }
    }
    await sampleProcess("after-sustained");
    const workSeconds = (performance.now() - began) / 1000;
    const workRecords = records.filter(record => record.pid === gallery.pid && record.receivedHostMs >= began);
    const completedImages = completions(records.filter(record => record.pid === gallery.pid)).filter(completed => completed.accepted && completed.request.source.startsWith(`http://127.0.0.1:${devicePort}/images/gallery-sustained/`));
    const distinctCompleted = [...new Map(completedImages.map(completed => [completed.request.source, completed])).values()];
    if (distinctCompleted.length < 9) throw Error(`Scrolling completed only ${distinctCompleted.length} distinct native decodes`);
    const distinctSubmitted = [...new Map(scrollingPixels.map(capture => [capture.artwork.index, capture])).values()];
    if (distinctSubmitted.length < 9) throw Error(`Scrolling verified only ${distinctSubmitted.length} distinct submitted artwork identities`);
    const measuredDistinctDecodedBytes = distinctCompleted.reduce((sum, completed) => sum + completed.decoded_bytes, 0);
    const pixelVerifiedSubmittedDecodedBytes = distinctSubmitted.reduce((sum, capture) => sum + capture.imageCompletion.decoded_bytes, 0);
    const decodedAndTextureCacheTargetBytes = 8 * 1024 * 1024;
    if (measuredDistinctDecodedBytes <= decodedAndTextureCacheTargetBytes || pixelVerifiedSubmittedDecodedBytes <= decodedAndTextureCacheTargetBytes) throw Error("The measured decoded and pixel-verified working sets must each exceed the 8 MiB cache target");
    const inputWindows = workRecords.map(record => record.message.match(/^InputUp ns=(\d+)$/)).filter(match => match !== null).map(match => Number(match![1]));
    if (inputWindows.length < 2 || !completedImages.some(completed => completed.accepted_ns > inputWindows[0]! && completed.accepted_ns < inputWindows.at(-1)!)) throw Error("No accepted native image arrivals occurred during the native-input workload interval");
    const responseReadinessInAdbWindows = requests.filter(request => request.path.startsWith("/images/gallery-sustained/") && request.respondedHostMs !== undefined
      && swipes.some(swipe => request.respondedHostMs! >= swipe.hostBeforeMs && request.respondedHostMs! <= swipe.hostAfterMs));
    // Wait for image/network/scroll settling before defining the idle period.
    let lastFrameCount = -1, stable = 0;
    await waitFor("post-work renderer settling", () => {
      const count = frames(records.filter(record => record.pid === gallery.pid)).length;
      stable = count === lastFrameCount ? stable + 1 : 0; lastFrameCount = count;
      return stable >= 20 ? true : undefined;
    });
    await sampleProcess("settled-after-work");
    result.postWorkIdle = await observeIdle("idle-after", idleSeconds);
    await sampleProcess("after-idle");
    const revisit = await settle(await open("network", 0, "gallery-sustained"), "cache-revisit", 0);
    result.sustained = { requestedSeconds: durationSeconds, elapsedSeconds: workSeconds, swipes, scrollingPixels, skippedCaptures, completedImages, distinctCompletedNativeSources: distinctCompleted.length, distinctPixelVerifiedSubmittedArtworks: distinctSubmitted.length, nativeInputUpNs: inputWindows, responseReadinessInAdbWindows,
      measuredDistinctDecodedBytes, pixelVerifiedSubmittedDecodedBytes, decodedAndTextureCacheTargetBytes,
      frames: frames(workRecords), rendererPerf: perf(workRecords), rendererMemory: memory(workRecords), revisit,
      cacheBoundary: "Only accepted native completions contribute decoded bytes; cancelled/host-ready responses do not. At least nine distinct pixel-verified artwork identities and their native decodes must exceed 8 MiB. Each counted screenshot has an independent complete artwork pixel match and a matching native completion included in a submission observed before capture. Before/after submissions bound observations; moving screenshots are not assigned to an exact submitted frame or scroll offset. Native completion timestamps overlap the interval between actual InputUp markers; per-swipe host bounds remain ADB command windows, not touch/arrival timing. Controlled 16 MiB traversal/revisit separately proves redecoding after retention turnover. Perf cache_misses also contains glyph misses." };
    result.processSamples = processSamples; result.cpuIntervals = cpuIntervals(processSamples); result.energyIntervals = energyIntervals(processSamples);
    result.energy = { status: emulator ? "unavailable" : processSamples.some(sample => sample.energy.status === "available") ? "available" : "unavailable",
      reason: emulator ? "No physical phone supplied; emulator energy/battery readings are excluded" : "See raw per-sample physical counters and charging state; no app-attributed energy is inferred", scope: "whole-device", physicalDeviceRequired: true };
    result.status = "passed";
  } catch (error) {
    result.status = "failed"; result.errors.push(String(error)); process.exitCode = 1;
  } finally {
    server?.stop(true);
    if (recorder) { kill(recorder); await recorder.exited; }
    if (recording) try { await recording; } catch (error) { result.errors.push(`Recorder: ${String(error)}`); }
    for (const [needed, name, args] of [
      [installed, "stop-fixture", ["shell", "am", "force-stop", packageName]],
      [installed, "uninstall", ["uninstall", packageName]],
      [reversed, "remove-reverse", ["reverse", "--remove", `tcp:${devicePort}`]],
    ] as const) if (needed) try { await adb([...args], name, true); } catch (error) { result.errors.push(`Cleanup ${name}: ${String(error)}`); }
    if (locked) await rm(lock, { recursive: true, force: true });
    if (result.errors.length) { result.status = "failed"; process.exitCode = 1; }
    await json("requests.json", { version: 1, requests });
    await json("result.json", { ...result, finishedAt: new Date().toISOString(), cleanup: { fixtureUninstalled: installed && !result.errors.some(error => error.startsWith("Cleanup uninstall")), reverseRemoved: reversed && !result.errors.some(error => error.startsWith("Cleanup remove-reverse")), deviceSettingsChanged: false, emulatorStopped: false } });
    console.log(`${result.status}: ${resolve(output, "result.json")}`);
    if (result.errors.length) console.error(result.errors.join("\n"));
  }
}

if (values.phase === "build") await build();
else await run();
