import { parseArgs } from "node:util";
import { chmod, copyFile, mkdir } from "node:fs/promises";
import { resolve } from "node:path";
import { arch, cpus, platform, release } from "node:os";

const root = resolve(import.meta.dir, "../..");
const fixtures = ["react", "list", "rows", "navigation", "conversation", "playback"];
const expectedWorkloads = [
  ...["small", "bulk", "resize"].map(command => `react-${command}`),
  ...[100, 1000, 10000].flatMap((count) => [`list-edit-${count}`, `list-scroll-${count}`]),
  "scrollbar-drag-10000",
  "rows-edit-100",
  "rows-edit-1000",
  "rows-prepend-1000",
  "rows-reorder-1000",
  "rows-native-scroll-1000",
  "rows-react-scroll-1000",
  "rows-decoded-image-arrival",
  "startup-runtime-to-library",
  "navigation-open",
  "navigation-back",
  "navigation-tab",
  "conversation-controlled-input",
  "conversation-append",
  "conversation-prepend",
  "conversation-input-background",
  "conversation-input-under-load",
  "playback-native-clock",
  "playback-state-change",
];
const { values } = parseArgs({
  options: {
    rounds: { type: "string", default: "5" },
    samples: { type: "string", default: "100" },
    warmup: { type: "string", default: "20" },
    baseline: { type: "string" },
    help: { type: "boolean" },
  },
});
if (values.help) {
  console.log(
    "Usage: bun run bench [--rounds 5] [--samples 100] [--warmup 20] [--baseline RUN_DIRECTORY]",
  );
  console.log(
    "Release QuickJS/scene benchmarks. Baseline runs alternate with the candidate; timings do not impose an automatic speed gate.",
  );
  process.exit(0);
}
const rounds = Number(values.rounds),
  samples = Number(values.samples),
  warmup = Number(values.warmup);
if (!Number.isInteger(rounds) || rounds < 1 || rounds > 50) throw Error("Rounds must be 1–50");
if (!Number.isInteger(samples) || samples < 2 || samples > 10000 || samples % 2 !== 0)
  throw Error("Samples must be even, from 2–10000");
if (!Number.isInteger(warmup) || warmup < 0 || warmup > 10000 || warmup % 2 !== 0)
  throw Error("Warmup must be even, from 0–10000");
const output = resolve(
  root,
  ".test-output/performance",
  new Date().toISOString().replaceAll(":", "-") + `-${process.pid}`,
);
await mkdir(output, { recursive: true });
console.log(`Performance evidence: ${output}`);

function git(...args: string[]) {
  const result = Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) throw Error(result.stderr.toString());
  return result.stdout.toString();
}
const hash = (bytes: Uint8Array | string) =>
  new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
const files = git("ls-files", "--cached", "--others", "--exclude-standard", "-z")
  .split("\0")
  .filter(Boolean);
const sourceHashes: Record<string, string> = {};
for (const file of new Set(files)) {
  const source = Bun.file(resolve(root, file));
  if (await source.exists()) sourceHashes[file] = hash(await source.bytes());
}
await Bun.write(
  resolve(output, "source-hashes.json"),
  JSON.stringify(sourceHashes, null, 2) + "\n",
);
await Bun.write(resolve(output, "tracked-changes.patch"), git("diff", "--binary", "HEAD"));
const workloadSources = Object.entries(sourceHashes)
  .filter(
    ([file]) =>
      file.startsWith("test/fixtures/performance/") ||
      (file.startsWith("test/performance/") && file.endsWith(".rs")),
  )
  .sort(([left], [right]) => left.localeCompare(right));
const workloadHash = hash(JSON.stringify(workloadSources));
const machine = { platform: platform(), arch: arch(), os: release(), cpu: cpus()[0]?.model };
const bundle = resolve(output, "bundle");

interface Bundle {
  workloadHash: string;
  machine: typeof machine;
  binarySha256: string;
  assets: Record<string, string>;
}
const baseline = values.baseline ? resolve(values.baseline) : undefined;
let baselineBundle: Bundle | undefined;
if (baseline) {
  const report = await Bun.file(resolve(baseline, "result.json")).json();
  if (report.version !== 1 || report.status !== "passed")
    throw Error("Baseline must be a completed successful benchmark run");
  baselineBundle = report.bundle;
  if (!baselineBundle || baselineBundle.workloadHash !== workloadHash)
    throw Error("Baseline workload source differs; establish a new baseline");
  if (JSON.stringify(baselineBundle.machine) !== JSON.stringify(machine))
    throw Error("Baseline machine differs; use a baseline from this host");
  await verifyBundle(resolve(baseline, "bundle"), baselineBundle);
}

async function build(name: string, argv: string[]) {
  const child = Bun.spawn(argv, { cwd: root, stdout: "pipe", stderr: "pipe" });
  const [code, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  await Bun.write(resolve(output, `${name}.log`), stdout + stderr);
  if (code !== 0) throw Error(`${name} failed; see ${output}/${name}.log\n${stderr}`);
}
console.log("Preparing production-compiled fixtures and release benchmark binary…");
await build("prepare", [
  process.execPath,
  "test/native/prepare.ts",
  "performance",
  "--profile",
  "release",
]);
await build("build", ["cargo", "build", "--release", "-p", "ink-test", "--bin", "ink-performance"]);
await mkdir(bundle, { recursive: true });
await copyFile(resolve(root, "target/release/ink-performance"), resolve(bundle, "ink-performance"));
await chmod(resolve(bundle, "ink-performance"), 0o755);
const assetHashes: Record<string, string> = {};
for (const fixture of fixtures) {
  await mkdir(resolve(bundle, "assets", fixture), { recursive: true });
  for (const file of ["app.js", "ink-icons-v1.bin"]) {
    const relative = `assets/${fixture}/${file}`;
    await copyFile(
      resolve(root, "test/fixtures/performance", fixture, ".ink/android/assets", file),
      resolve(bundle, relative),
    );
    assetHashes[relative] = hash(await Bun.file(resolve(bundle, relative)).bytes());
  }
}
const candidateBundle: Bundle = {
  workloadHash,
  machine,
  assets: assetHashes,
  binarySha256: hash(await Bun.file(resolve(bundle, "ink-performance")).bytes()),
};

async function verifyBundle(directory: string, description: Bundle) {
  if (
    hash(await Bun.file(resolve(directory, "ink-performance")).bytes()) !== description.binarySha256
  )
    throw Error("Saved benchmark binary changed");
  for (const [file, expected] of Object.entries(description.assets)) {
    if (hash(await Bun.file(resolve(directory, file)).bytes()) !== expected)
      throw Error(`Saved benchmark asset changed: ${file}`);
  }
}

interface Sample {
  elapsed_ns: number;
  apply_ns?: number;
  dispatch_lateness_ns?: number;
  dispatch_to_ack_ns?: number;
  input_dispatch_ns?: number;
  thumb_update_ns?: number;
  nodes_measured: number;
  full_rebuilds: number;
}
interface Workload {
  name: string;
  samples: Sample[];
  contracts: unknown;
}
interface Round {
  version: number;
  workloads: Workload[];
  idle: { name: string; commits: number };
}
const candidateRounds: Round[] = [],
  baselineRounds: Round[] = [];
const executionOrder: string[] = [];
const runStart = performance.now();
async function runRound(label: string, index: number, directory: string) {
  executionOrder.push(`${label}-${index}`);
  const child = Bun.spawn(
    [resolve(directory, "ink-performance"), String(samples), String(warmup)],
    {
      cwd: root,
      stdout: "pipe",
      stderr: "pipe",
      env: { ...process.env, INK_PERF_ASSETS: resolve(directory, "assets") },
    },
  );
  const timeout = setTimeout(() => child.kill(), 120000);
  const [code, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  clearTimeout(timeout);
  await Bun.write(resolve(output, `${label}-${index}.json`), stdout);
  await Bun.write(resolve(output, `${label}-${index}.log`), stderr);
  if (code !== 0)
    throw Error(`${label} round ${index} failed; see ${output}/${label}-${index}.log\n${stderr}`);
  const report: Round = JSON.parse(stdout);
  if (report.version !== 1 || report.idle.commits !== 0) throw Error("Unexpected benchmark report");
  if (
    JSON.stringify(report.workloads.map((workload) => workload.name)) !==
    JSON.stringify(expectedWorkloads)
  )
    throw Error("Unexpected workload names or order");
  for (const workload of report.workloads) {
    if (
      workload.samples.length !== samples ||
      workload.samples.some(
        (sample) => !Number.isFinite(sample.elapsed_ns) || sample.elapsed_ns <= 0,
      )
    )
      throw Error(`Invalid samples: ${workload.name}`);
  }
  console.log(
    `${label} round ${index + 1}/${rounds}: ${expectedWorkloads.length} workloads and idle contract passed`,
  );
  return report;
}
try {
  for (let index = 0; index < rounds; index++) {
    const order =
      baseline && index % 2 === 0
        ? ["baseline", "candidate"]
        : baseline
          ? ["candidate", "baseline"]
          : ["candidate"];
    for (const label of order) {
      if (label === "baseline")
        baselineRounds.push(await runRound(label, index, resolve(baseline!, "bundle")));
      else candidateRounds.push(await runRound(label, index, bundle));
    }
  }
} catch (error) {
  await Bun.write(
    resolve(output, "result.json"),
    JSON.stringify(
      {
        version: 1,
        status: "failed",
        bundle: candidateBundle,
        error: String(error),
        executionOrder,
      },
      null,
      2,
    ) + "\n",
  );
  throw error;
}

function distribution(samples: number[]) {
  const sorted = [...samples].sort((left, right) => left - right);
  const percentile = (fraction: number) => sorted[Math.ceil(sorted.length * fraction) - 1] / 1e6;
  return {
    count: sorted.length,
    mean_ms: sorted.reduce((sum, value) => sum + value, 0) / sorted.length / 1e6,
    median_ms: percentile(0.5),
    p95_ms: percentile(0.95),
    p99_ms: percentile(0.99),
  };
}
function summarise(reports: Round[]) {
  return expectedWorkloads.map((name, index) => {
    const perRound = reports.map((report) => report.workloads[index]);
    const phase = (field: keyof Sample) => {
      const times = perRound.flatMap((workload) => workload.samples.map((sample) => sample[field]));
      return times.every((value) => value !== undefined) ? distribution(times) : undefined;
    };
    return {
      name,
      ...distribution(
        perRound.flatMap((workload) => workload.samples.map((sample) => sample.elapsed_ns)),
      ),
      round_median_ms: perRound.map(
        (workload) => distribution(workload.samples.map((sample) => sample.elapsed_ns)).median_ms,
      ),
      apply: phase("apply_ns"),
      dispatch_lateness: phase("dispatch_lateness_ns"),
      dispatch_to_ack: phase("dispatch_to_ack_ns"),
      input_dispatch: phase("input_dispatch_ns"),
      thumb_update: phase("thumb_update_ns"),
      mean_nodes_measured:
        perRound
          .flatMap((workload) => workload.samples)
          .reduce((sum, sample) => sum + sample.nodes_measured, 0) /
        (reports.length * samples),
    };
  });
}
const candidateSummary = summarise(candidateRounds),
  baselineSummary = baseline ? summarise(baselineRounds) : undefined;
const comparison = baselineSummary?.map((previous, index) => {
  const current = candidateSummary[index];
  return {
    name: current.name,
    median_change_percent: (current.median_ms / previous.median_ms - 1) * 100,
    p95_change_percent: (current.p95_ms / previous.p95_ms - 1) * 100,
    round_median_change_percent: current.round_median_ms.map(
      (value, round) => (value / previous.round_median_ms[round] - 1) * 100,
    ),
  };
});
const result = {
  version: 1,
  status: "passed",
  revision: git("rev-parse", "HEAD").trim(),
  dirty: git("status", "--porcelain").length > 0,
  profile: "release",
  bun: Bun.version,
  rustc: Bun.spawnSync(["rustc", "--version"], { stdout: "pipe" }).stdout.toString().trim(),
  rounds,
  samples,
  warmup,
  executionSeconds: (performance.now() - runStart) / 1000,
  bundle: candidateBundle,
  baseline: baseline ? { directory: baseline, bundle: baselineBundle } : undefined,
  executionOrder,
  summary: candidateSummary,
  baselineSummary,
  comparison,
  completedAt: new Date().toISOString(),
};
await Bun.write(resolve(output, "result.json"), JSON.stringify(result, null, 2) + "\n");
const rows = candidateSummary
  .map(
    (row) =>
      `| ${row.name} | ${row.median_ms.toFixed(3)} | ${row.p95_ms.toFixed(3)} | ${row.p99_ms.toFixed(3)} | ${row.mean_nodes_measured.toFixed(1)} |`,
  )
  .join("\n");
const inputRows = candidateSummary.flatMap(
  ({ name, dispatch_lateness, dispatch_to_ack, input_dispatch }) =>
    dispatch_lateness && dispatch_to_ack && input_dispatch
      ? [
          `| ${name} | ${dispatch_lateness.p95_ms.toFixed(3)} | ${dispatch_to_ack.p95_ms.toFixed(3)} | ${input_dispatch.p95_ms.toFixed(3)} |`,
        ]
      : [],
);
const inputPhaseTable = `
Scheduled-input phase timings distinguish late host dispatch from subsequent runtime/scene acknowledgement. Input dispatch includes native editing, event construction and enqueueing; it is not Android IME latency.

| Workload | Dispatch lateness p95 ms | Dispatch to acknowledgement p95 ms | Input dispatch p95 ms |
| --- | ---: | ---: | ---: |
${inputRows.join("\n")}
`;
const comparisonTable = comparison
  ? `
Baseline and candidate were rerun in alternating order. Negative changes mean faster. Per-round changes and raw samples remain in result.json.

| Workload | Median change % | p95 change % |
| --- | ---: | ---: |
${comparison.map((row) => `| ${row.name} | ${row.median_change_percent.toFixed(1)} | ${row.p95_change_percent.toFixed(1)} |`).join("\n")}
`
  : "";
const thumbRows = candidateSummary.flatMap(({ name, thumb_update, median_ms }) =>
  thumb_update
    ? [
        `| ${name} | ${thumb_update.median_ms.toFixed(3)} | ${thumb_update.p95_ms.toFixed(3)} | ${thumb_update.p99_ms.toFixed(3)} | ${median_ms.toFixed(3)} |`,
      ]
    : [],
);
const thumbTable = `
Thumb update measures pointer movement through the native thumb geometry used by the renderer. Ready-scene timing also includes row window materialisation. GPU submission and display presentation are excluded.

| Workload | Thumb median ms | Thumb p95 ms | Thumb p99 ms | Ready-scene median ms |
| --- | ---: | ---: | ---: | ---: |
${thumbRows.join("\n")}
`;
const report = `# Headless performance

Release build on ${machine.cpu} (${machine.platform}/${machine.arch}). ${rounds} fresh processes, ${warmup} warm-ups and ${samples} measured operations per workload/process.

| Workload | Median ms | p95 ms | p99 ms | Mean nodes measured |
| --- | ---: | ---: | ---: | ---: |
${rows}
${inputPhaseTable}
${thumbTable}
${comparisonTable}
All workload correctness/work contracts and idle observations passed. Updates cover dispatch through native scene application. Navigation includes native hit testing/back and viewport readiness. Controlled input covers native editing through the JS acknowledgement applied to the scene. Loaded input includes waiting from its prescribed arrival time. Scroll samples include JS window materialisation for custom React rows. Startup covers a fresh runtime/engine with source already read; image arrival starts with decoded RGBA; playback clocks bypass JS. These timings exclude Android process launch, IME, services, image decoding, GPU work and presentation. Native core instrumentation is enabled. Full correctness assertions run outside samples; scene readiness detection remains inside the measured path.

This is a dirty-tree measurement with source hashes, not a committed-source reproduction claim. The saved executable and compiled assets allow alternating baseline comparisons. Timing changes are reported without an automatic speed threshold.
`;
await Bun.write(resolve(output, "report.md"), report);
await Bun.write(
  resolve(root, ".test-output/performance/latest.json"),
  JSON.stringify({ directory: output }, null, 2) + "\n",
);
console.log(`\n${report}\nEvidence: ${output}`);
