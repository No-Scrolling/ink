import { parseArgs } from "node:util";
import { chmod, copyFile, mkdir } from "node:fs/promises";
import { arch, cpus, homedir, loadavg, platform, release } from "node:os";
import { dirname, resolve } from "node:path";
import { primaryMetric, selectSuites, suites, validateRun, workloadNames, workloadSamples, type Run, type Sample, type Selection, type Suite } from "./suites";

const root = resolve(import.meta.dir, "../..");
const { values } = parseArgs({ options: {
  phase: { type: "string", default: "all" },
  suite: { type: "string" },
  output: { type: "string" },
  rounds: { type: "string", default: "3" },
  samples: { type: "string", default: "20" },
  warmup: { type: "string", default: "4" },
  serial: { type: "string" },
  "duration-seconds": { type: "string", default: "120" },
  list: { type: "boolean" },
  help: { type: "boolean" },
} });
if (values.help) {
  console.log("Usage: bun run bench [--suite all|core|automatic|journeys|renderer|coverage|device] [--phase all|build|run] [--output DIRECTORY] [--rounds 3] [--samples 20] [--warmup 4] [--serial SERIAL] [--duration-seconds 120] [--list]");
  console.log("Builds finish before serial release measurements. --serial adds Android; without it, Android is explicitly not requested. Core requires even sample and warm-up counts.");
  console.log("Use bench:core --baseline for saved core comparisons, or bench:compare:automatic for paired specialist comparisons.");
  process.exit(0);
}
if (values.list) {
  const selection = selectSuites(values.suite ?? "coverage");
  let total = 0;
  for (const suite of selection.suites) {
    const names = workloadNames(suite, selection.coverageOnly);
    total += names.length;
    console.log(`${suite} (${names.length})\n${names.map(name => `  ${name}`).join("\n")}`);
  }
  console.log(`${total} host/renderer workloads. Android media, lifecycle and sustained usage run with --serial. Physical energy needs eligible hardware.`);
  process.exit(0);
}
const phase = values.phase!;
if (!["all", "build", "run"].includes(phase)) throw Error("Expected --phase all, build or run");
if (phase === "run" && !values.output) throw Error("--phase run requires the directory produced by --phase build");
function integer(value: string, name: string, minimum: number, maximum: number): number {
  const number = Number(value);
  if (!Number.isInteger(number) || number < minimum || number > maximum) throw Error(`${name} must be ${minimum}–${maximum}`);
  return number;
}
const rounds = integer(values.rounds!, "Rounds", 1, 20);
const samples = integer(values.samples!, "Samples", 2, 1000);
const warmup = integer(values.warmup!, "Warm-up", 0, 1000);
const duration = integer(values["duration-seconds"]!, "Duration", 120, 3600);
const output = resolve(values.output ?? resolve(root, ".test-output/benchmarks", new Date().toISOString().replaceAll(":", "-") + `-${process.pid}`));
const bundle = resolve(output, "bundle");
const machine = { platform: platform(), arch: arch(), os: release(), cpu: cpus()[0]?.model };
type Manifest = {
  version: number;
  machine: typeof machine;
  hashes: Record<string, string>;
  serial?: string;
  selection?: Selection;
  rendererLibrary?: { path: string; sha256: string } | null;
};
const saved: Manifest | undefined = phase === "run" ? await Bun.file(resolve(output, "bundle.json")).json() : undefined;
const selection = values.suite ? selectSuites(values.suite) : saved?.selection ?? selectSuites("coverage");
if (saved && JSON.stringify(selection) !== JSON.stringify(saved.selection ?? selectSuites("coverage"))) throw Error("Requested suites differ from the saved build; use its original selection or build a new output directory");
const serial = values.serial ?? saved?.serial;
if (serial && samples > 48) throw Error("Samples must be 2–48 when Android workloads are included");
if (!selection.suites.length && !serial) throw Error("The device suite requires --serial SERIAL");
if (saved && serial && !saved.serial) throw Error("The saved build has no Android APK; build a new output directory with --serial");
if (selection.suites.includes("core") && (samples % 2 || warmup % 2)) throw Error("Core requires even sample and warm-up counts");
await mkdir(output, { recursive: true });
console.log(`Benchmark evidence: ${output}`);
console.log(`Suites: ${selection.suites.join(", ") || "device only"}. Android: ${serial ?? "not requested (use --serial)"}`);
const hash = (bytes: Uint8Array | string) => new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
function git(...args: string[]): string {
  const result = Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (result.exitCode) throw Error(result.stderr.toString());
  return result.stdout.toString();
}
async function command(name: string, argv: string[], env = process.env, timeoutMs = 900_000): Promise<string> {
  console.log(`${name}…`);
  const child = Bun.spawn(argv, { cwd: root, env, detached: true, stdout: "pipe", stderr: "pipe" });
  const timer = setTimeout(() => {
    try { process.kill(-child.pid, "SIGKILL"); }
    catch (error) { if (!(error instanceof Error && "code" in error && error.code === "ESRCH")) console.error(error); }
  }, timeoutMs);
  try {
    const [code, stdout, stderr] = await Promise.all([child.exited, new Response(child.stdout).text(), new Response(child.stderr).text()]);
    await Bun.write(resolve(output, `${name}.stdout`), stdout);
    await Bun.write(resolve(output, `${name}.stderr`), stderr);
    if (code) throw Error(`${name} failed (${code}); see ${output}/${name}.stderr`);
    return stdout;
  } finally { clearTimeout(timer); }
}
function noBuilds(): void {
  const result = Bun.spawnSync(["ps", "-Ao", "comm"], { stdout: "pipe", stderr: "pipe" });
  if (result.exitCode) throw Error("Cannot verify that compilation has finished");
  const builds = result.stdout.toString().split("\n").filter(line => ["cargo", "rustc", "clang", "clang++", "javac"].includes(line.trim().split("/").at(-1)!));
  if (builds.length) throw Error(`Compilation is active; finish it before timing: ${builds.join(", ")}`);
}
if (phase !== "run") {
  if (await Bun.file(resolve(output, "bundle.json")).exists()) throw Error("Output already contains a build; choose a new directory to preserve it");
  const rendererDirectory = process.env.INK_DESIGN_RENDERER_DIR ?? resolve(process.env.XDG_CACHE_HOME ?? resolve(homedir(), ".cache"), "ink/design-renderer/mesa-3fc652a060");
  const rendererPath = resolve(process.env.INK_VULKAN_LIBRARY ?? resolve(rendererDirectory, "lib", platform() === "darwin" ? "libvulkan_lvp.dylib" : platform() === "win32" ? "vulkan_lvp.dll" : "libvulkan_lvp.so"));
  const usesRenderer = selection.suites.includes("renderer");
  if (usesRenderer && !await Bun.file(rendererPath).exists()) throw Error("Set INK_VULKAN_LIBRARY or prepare the runtime with scripts/setup-design-renderer before building renderer benchmarks");
  const binaries = [...new Set(selection.suites.map(suite => suites[suite].binary))];
  const assets = [...new Set(selection.suites.flatMap(suite => suites[suite].assets))];
  if (binaries.length) {
    await command("prepare", [process.execPath, "test/native/prepare.ts", "performance", "--profile", "release"]);
    await command("build-host", ["cargo", "build", "--release", "-p", "ink-test", ...binaries.flatMap(name => ["--bin", name])]);
  }
  await mkdir(bundle, { recursive: true });
  const hashes: Record<string, string> = {};
  for (const name of binaries) {
    await copyFile(resolve(root, "target/release", name), resolve(bundle, name));
    await chmod(resolve(bundle, name), 0o755);
    hashes[name] = hash(await Bun.file(resolve(bundle, name)).bytes());
  }
  for (const fixture of assets) {
    await mkdir(resolve(bundle, "assets", fixture), { recursive: true });
    for (const file of ["app.js", "ink-icons-v1.bin"]) {
      const path = `assets/${fixture}/${file}`;
      await copyFile(resolve(root, "test/fixtures/performance", fixture, ".ink/android/assets", file), resolve(bundle, path));
      hashes[path] = hash(await Bun.file(resolve(bundle, path)).bytes());
    }
  }
  if (serial) await command("build-device", [process.execPath, "test/performance/device-run.ts", "--phase", "build", "--serial", serial, "--output", resolve(output, "device"), "--samples", String(samples), "--duration-seconds", String(duration)], process.env, 1_860_000);
  const sources: Record<string, string> = {};
  const files = git("ls-files", "--cached", "--others", "--exclude-standard", "-z").split("\0").filter(Boolean);
  for (const path of new Set(files)) {
    if (!/\.(rs|ts|tsx|js|toml|json|lock|kt|kts|wgsl)$/.test(path)) continue;
    const file = Bun.file(resolve(root, path));
    if (await file.exists()) {
      sources[path] = hash(await file.bytes());
      const destination = resolve(output, "sources", path);
      await mkdir(dirname(destination), { recursive: true });
      await copyFile(resolve(root, path), destination);
    }
  }
  await Bun.write(resolve(output, "tracked-changes.patch"), git("diff", "--binary", "HEAD"));
  const rustc = Bun.spawnSync(["rustc", "--version"], { stdout: "pipe", stderr: "pipe" });
  if (rustc.exitCode) throw Error("Cannot record the Rust toolchain");
  await Bun.write(resolve(output, "bundle.json"), JSON.stringify({ version: 2, selection, machine, revision: git("rev-parse", "HEAD").trim(), dirty: !!git("status", "--porcelain"), hashes, sources, serial,
    toolchain: { bun: Bun.version, rustc: rustc.stdout.toString().trim() },
    rendererLibrary: usesRenderer ? { path: rendererPath, sha256: hash(await Bun.file(rendererPath).bytes()) } : null,
    builtAt: new Date().toISOString() }, null, 2) + "\n");
}
if (phase === "build") process.exit(0);
const manifest: Manifest = saved ?? await Bun.file(resolve(output, "bundle.json")).json();
if (JSON.stringify(manifest.machine) !== JSON.stringify(machine)) throw Error("The saved bundle was built on a different host");
for (const [path, expected] of Object.entries(manifest.hashes)) {
  if (hash(await Bun.file(resolve(bundle, path)).bytes()) !== expected) throw Error(`Saved bundle changed: ${path}`);
}
if (selection.suites.includes("renderer") && (!manifest.rendererLibrary || hash(await Bun.file(manifest.rendererLibrary.path).bytes()) !== manifest.rendererLibrary.sha256)) throw Error("The saved Vulkan runtime library changed or was not recorded");
const runs: { group: Suite; round: number; hostLoad: number[]; data: Run }[] = [];
let deviceResult: { status: string; energy: unknown } | undefined;
try {
  for (let round = 0; round < rounds; round++) {
    const order = round % 2 ? [...selection.suites].reverse() : selection.suites;
    for (const group of order) {
      noBuilds();
      const hostLoad = loadavg();
      const env = { ...process.env,
        INK_VULKAN_LIBRARY: manifest.rendererLibrary?.path ?? process.env.INK_VULKAN_LIBRARY,
        INK_PERF_ASSETS: resolve(bundle, "assets"), INK_AUTOMATIC_ASSETS: resolve(bundle, "assets/automatic"),
        INK_PERF_RENDER_ARTIFACTS: resolve(output, `renderer-${round}-pixels`) };
      const args = [String(samples), String(warmup)];
      if (group === "automatic") args.push("1000,5000");
      if (group === "renderer" && selection.coverageOnly) args.unshift("--coverage-only");
      const stdout = await command(`${group}-${round}`, [resolve(bundle, suites[group].binary), ...args], env);
      noBuilds();
      const data: Run = JSON.parse(stdout);
      validateRun(group, data, samples, selection.coverageOnly);
      runs.push({ group, round, hostLoad, data });
      await Bun.write(resolve(output, `${group}-${round}.json`), JSON.stringify(data, null, 2) + "\n");
    }
  }
  if (serial) {
    noBuilds();
    const timeoutMs = Math.max(900_000, (duration + samples * 6 * 90 + 300) * 1000);
    await command("run-device", [process.execPath, "test/performance/device-run.ts", "--phase", "run", "--serial", serial, "--output", resolve(output, "device"), "--samples", String(samples), "--duration-seconds", String(duration)], process.env, timeoutMs);
    noBuilds();
    deviceResult = await Bun.file(resolve(output, "device/result.json")).json();
    if (deviceResult?.status !== "passed") throw Error("Android workloads did not complete successfully");
  }
} catch (error) {
  await Bun.write(resolve(output, "result.json"), JSON.stringify({ version: 2, status: "failed", selection, error: String(error), executionOrder: runs.map(({group, round}) => ({group, round})), failedAt: new Date().toISOString() }, null, 2) + "\n");
  throw error;
}
function statistics(values: number[]) {
  const sorted = values.toSorted((a, b) => a - b);
  return { n: sorted.length, median: sorted[Math.ceil(sorted.length * .5) - 1], p95: sorted[Math.ceil(sorted.length * .95) - 1], p99: sorted[Math.ceil(sorted.length * .99) - 1], min: sorted[0], max: sorted.at(-1) };
}
function summarise(samples: Sample[]) {
  const metrics: Record<string, ReturnType<typeof statistics>> = {};
  for (const field of Object.keys(samples[0] ?? {})) {
    const numbers = samples.map(sample => sample[field]);
    if (numbers.every((value): value is number => typeof value === "number" && Number.isFinite(value))) metrics[field] = statistics(numbers);
  }
  return metrics;
}
const summary = [];
for (const group of selection.suites) {
  const blocks = runs.filter(run => run.group === group);
  for (const name of workloadNames(group, selection.coverageOnly)) {
    const workloads = blocks.map(block => block.data.workloads.find(workload => workload.name === name)!);
    const inputs = workloads.flatMap(workload => workload.jobs?.flatMap(job => job.inputs) ?? []);
    summary.push({ group, name, primaryMetric: primaryMetric(group, workloads[0]),
      metrics: summarise(workloads.flatMap(workloadSamples)), inputMetrics: inputs.length ? summarise(inputs) : undefined,
      acknowledgementsBeforeListReady: inputs.length ? inputs.filter(input => input.before_list_ready).length : undefined });
  }
}
const result = { version: 2, status: "passed", selection, machine, rounds, samples, warmup,
  executionOrder: runs.map(({group, round, hostLoad}) => ({group, round, hostLoad})), summary,
  device: serial ? { directory: "device", serial, status: deviceResult!.status, energy: deviceResult!.energy } : { status: "not_requested", reason: "Supply --serial to run Android workloads" },
  boundaries: { host: "Release QuickJS and native scene application; excludes Android presentation", renderer: "Offscreen Vulkan device named in raw results; CPU submission and readback are separate metrics", energy: "Requires genuine physical-device counters; emulator battery values are not measurements" }, completedAt: new Date().toISOString() };
await Bun.write(resolve(output, "result.json"), JSON.stringify(result, null, 2) + "\n");
const rows = summary.map(({group, name, primaryMetric, metrics}) => {
  const metric = metrics[primaryMetric];
  return `| ${group} | ${name} | ${primaryMetric} | ${(metric.median / 1e6).toFixed(3)} | ${(metric.p95 / 1e6).toFixed(3)} | ${metric.n} |`;
});
const typingRows = summary.filter(row => row.inputMetrics).map(row => {
  const metric = row.inputMetrics!.arrival_to_ack_ns;
  return `| ${row.name} | ${(metric.median / 1e6).toFixed(3)} | ${(metric.p95 / 1e6).toFixed(3)} | ${metric.n} | ${row.acknowledgementsBeforeListReady} |`;
});
const typing = typingRows.length ? `\n## Typing during automatic refresh\n\nInput acknowledgements and final list readiness have separate endpoints.\n\n| Workload | Arrival-to-ack median ms | p95 ms | Inputs | Acknowledged before list ready |\n| --- | ---: | ---: | ---: | ---: |\n${typingRows.join("\n")}\n` : "";
await Bun.write(resolve(output, "report.md"), `# Performance benchmark run\n\n${rounds} fresh processes per host suite. Times are milliseconds; the metric column names each endpoint. Renderer CPU submission excludes completion/readback, which remains in the raw results. Automatic typing reports both list readiness and individual input acknowledgements. Raw workload contracts, caps and floors remain in each suite JSON.\n\n| Suite | Workload | Metric | Median ms | p95 ms | Samples |\n| --- | --- | --- | ---: | ---: | ---: |\n${rows.join("\n")}\n${typing}\n${serial ? "Android results and physical-energy availability are in device/result.json." : "Android workloads were not requested; use --serial."}\n\nPaired historical comparisons remain available through bench:core --baseline and bench:compare:automatic. This current-build run does not claim a before/after optimisation.\n`);
console.log(`Completed ${summary.length} host/renderer workloads. Android: ${deviceResult?.status ?? "not requested"}. Evidence: ${output}`);
