import { parseArgs } from "node:util";
import { chmod, copyFile, lstat, mkdir, readlink, symlink } from "node:fs/promises";
import { basename, dirname, resolve } from "node:path";
import { arch, cpus, platform, release } from "node:os";

const root = resolve(import.meta.dir, "../..");
const evidenceRoot = resolve(root, ".test-output/native-source/2026-10-22");
const binaryName = "ink-source-performance";
const harnessPath = "test/performance/source.rs";
const manifestPath = "test/native/Cargo.toml";
const registration =
  '\n[[bin]]\nname = "ink-source-performance"\npath = "../performance/source.rs"\n';
const { values } = parseArgs({
  options: {
    phase: { type: "string" },
    output: { type: "string" },
    "baseline-source": { type: "string", default: resolve(evidenceRoot, "baseline-host-source") },
    "baseline-revision": { type: "string", default: "dfef70e" },
    assets: {
      type: "string",
      default: resolve(root, "test/fixtures/performance/source/.ink/android/assets"),
    },
    blocks: { type: "string", default: "1" },
    samples: { type: "string", default: "64" },
    warmup: { type: "string", default: "12" },
    counts: { type: "string", default: "1000,5000" },
    "timeout-seconds": { type: "string", default: "600" },
    render: { type: "boolean", default: false },
    help: { type: "boolean" },
  },
});
if (values.help || !values.phase) {
  console.log(`Usage: bun test/performance/source-run.ts --phase build|run|all [--output DIRECTORY]

build: use the already prepared candidate fixture; snapshot sources/assets and build both runtimes sequentially.
run:   use --output from a completed build; never compile or prepare assets.
all:   finish both builds before starting any measurements.

Defaults: --blocks 1 --samples 64 --warmup 12 --counts 1000,5000
Each block starts four fresh processes: baseline/candidate/candidate/baseline, then the reverse order in the next block.
10,000 rows exceed the array baseline's 1 MiB commit input limit on Android.
100,000 is explicitly optional and exploratory: the same limit prevents a meaningful main comparison at that size.
--render saves the final 1080 × 1240 scene after all timed workloads in each process.
Finish other compilation before running measurements.`);
  process.exit(values.help ? 0 : 2);
}
if (!["build", "run", "all"].includes(values.phase))
  throw Error("Expected --phase build, run or all");
if (values.phase === "run" && !values.output)
  throw Error("The run phase requires --output from the build phase");
const output = resolve(
  values.output ??
    resolve(
      evidenceRoot,
      `benchmark-${new Date().toISOString().replaceAll(":", "-")}-${process.pid}`,
    ),
);
const machine = { platform: platform(), arch: arch(), os: release(), cpu: cpus()[0]?.model };
const hash = (bytes: Uint8Array | string) =>
  new Bun.CryptoHasher("sha256").update(bytes).digest("hex");

function git(...arguments_: string[]): string {
  const result = Bun.spawnSync(["git", ...arguments_], {
    cwd: root,
    stdout: "pipe",
    stderr: "pipe",
  });
  if (result.exitCode !== 0) throw Error(result.stderr.toString());
  return result.stdout.toString();
}

async function writeJson(path: string, value: unknown) {
  await Bun.write(path, JSON.stringify(value, null, 2) + "\n");
}

function integer(value: string, name: string, minimum: number, maximum: number) {
  const number = Number(value);
  if (!Number.isInteger(number) || number < minimum || number > maximum)
    throw Error(`${name} must be an integer from ${minimum}–${maximum}`);
  return number;
}

type SourceEntry = { kind: "file" | "symlink"; sha256: string };
type Sources = Record<string, SourceEntry>;
type Build = {
  version: number;
  status: string;
  createdAt: string;
  machine: typeof machine;
  profile: string;
  baseline: { revision: string; archive: string; binarySha256: string; sources: Sources };
  candidate: { revision: string; dirty: boolean; binarySha256: string; sources: Sources };
  harnessSha256: string;
  assets: Record<string, string>;
  rustc: string;
  cargo: string;
  bun: string;
};

function candidateFiles() {
  return [
    ...new Set(
      git("ls-files", "--cached", "--others", "--exclude-standard", "-z")
        .split("\0")
        .filter(Boolean),
    ),
  ].sort();
}

async function snapshotSources(
  directory: string,
  files: string[],
  destination?: string,
): Promise<Sources> {
  const entries: Sources = {};
  for (const file of files) {
    const source = resolve(directory, file);
    const stat = await lstat(source).catch((error: NodeJS.ErrnoException) => {
      if (error.code === "ENOENT") return undefined;
      throw error;
    });
    if (!stat) continue;
    const target = destination ? resolve(destination, file) : undefined;
    if (target) await mkdir(dirname(target), { recursive: true });
    if (stat.isSymbolicLink()) {
      const link = await readlink(source);
      entries[file] = { kind: "symlink", sha256: hash(link) };
      if (target) await symlink(link, target);
    } else if (stat.isFile()) {
      const bytes = await Bun.file(source).bytes();
      entries[file] = { kind: "file", sha256: hash(bytes) };
      if (target) await copyFile(source, target);
    } else throw Error(`Unexpected source entry: ${source}`);
  }
  return entries;
}

async function buildBinary(label: string, directory: string) {
  const target = resolve(root, "target");
  console.log(`Building ${label} release harness…`);
  const child = Bun.spawn(
    [
      "cargo",
      "build",
      "--locked",
      "--release",
      "-p",
      "ink-test",
      "--bin",
      binaryName,
      "--target-dir",
      target,
    ],
    { cwd: directory, stdout: "pipe", stderr: "pipe" },
  );
  const [code, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  await Bun.write(resolve(output, `build-${label}.log`), stdout + stderr);
  if (code !== 0) throw Error(`${label} build failed; see ${output}/build-${label}.log\n${stderr}`);
  const destination = resolve(output, "bundle", label, binaryName);
  await mkdir(dirname(destination), { recursive: true });
  await copyFile(resolve(target, "release", binaryName), destination);
  await chmod(destination, 0o755);
  return hash(await Bun.file(destination).bytes());
}

async function prepareBuild(): Promise<Build> {
  if (await Bun.file(resolve(output, "build.json")).exists())
    throw Error("This output already contains a build; choose a fresh --output directory");
  const baseline = resolve(values["baseline-source"]!);
  if (baseline === root) throw Error("The baseline must be the separate baseline archive");
  const revision = git("rev-parse", `${values["baseline-revision"]}^{commit}`).trim();
  const archived = git("ls-tree", "-rz", revision).split("\0").filter(Boolean);
  const baselineFiles: string[] = [];
  // Reject baseline edits apart from this identical harness and its bin registration.
  for (const entry of archived) {
    const [header, file] = entry.split("\t");
    const [mode, type, expected] = header.split(" ");
    if (type !== "blob") throw Error(`Unsupported baseline entry: ${entry}`);
    baselineFiles.push(file);
    let bytes =
      mode === "120000"
        ? new TextEncoder().encode(await readlink(resolve(baseline, file)))
        : await Bun.file(resolve(baseline, file)).bytes();
    if (file === manifestPath)
      bytes = new TextEncoder().encode(
        new TextDecoder().decode(bytes).replaceAll(registration, ""),
      );
    const actual = new Bun.CryptoHasher("sha1")
      .update(`blob ${bytes.length}\0`)
      .update(bytes)
      .digest("hex");
    if (actual !== expected) throw Error(`Baseline archive differs from ${revision}: ${file}`);
  }
  const candidateManifest = await Bun.file(resolve(root, manifestPath)).text();
  if (
    !candidateManifest
      .split("[[bin]]")
      .some(
        (block) =>
          /name\s*=\s*"ink-source-performance"/.test(block) &&
          /path\s*=\s*"\.\.\/performance\/source\.rs"/.test(block),
      )
  )
    throw Error(
      "Register ink-source-performance in test/native/Cargo.toml before building",
    );
  for (const file of ["app.js", "ink-icons-v1.bin"])
    if (!(await Bun.file(resolve(values.assets!, file)).exists()))
      throw Error(`Prepare the source fixture before building: ${resolve(values.assets!, file)}`);
  await mkdir(output, { recursive: true });
  const harness = await Bun.file(resolve(root, harnessPath)).bytes();
  await mkdir(dirname(resolve(baseline, harnessPath)), { recursive: true });
  await Bun.write(resolve(baseline, harnessPath), harness);
  const baselineManifest = await Bun.file(resolve(baseline, manifestPath)).text();
  await Bun.write(
    resolve(baseline, manifestPath),
    baselineManifest.replaceAll(registration, "") + registration,
  );
  if (!baselineFiles.includes(harnessPath)) baselineFiles.push(harnessPath);
  baselineFiles.sort();
  const files = candidateFiles();
  const baselineSources = await snapshotSources(
    baseline,
    baselineFiles,
    resolve(output, "source/baseline"),
  );
  const candidateSources = await snapshotSources(root, files, resolve(output, "source/candidate"));
  await Bun.write(resolve(output, "candidate-changes.patch"), git("diff", "--binary", "HEAD"));
  await Bun.write(
    resolve(output, "candidate-status.txt"),
    git("status", "--porcelain=v1", "--untracked-files=all"),
  );
  const assets: Record<string, string> = {};
  for (const file of ["app.js", "ink-icons-v1.bin"]) {
    const relative = `assets/source/${file}`;
    const destination = resolve(output, "bundle", relative);
    await mkdir(dirname(destination), { recursive: true });
    await copyFile(resolve(values.assets!, file), destination);
    assets[relative] = hash(await Bun.file(destination).bytes());
  }
  const toolVersion = (tool: string) => {
    const result = Bun.spawnSync([tool, "--version"], { stdout: "pipe", stderr: "pipe" });
    if (result.exitCode !== 0) throw Error(result.stderr.toString());
    return result.stdout.toString().trim();
  };
  const description: Build = {
    version: 1,
    status: "building",
    createdAt: new Date().toISOString(),
    machine,
    profile: "release",
    baseline: { revision, archive: baseline, binarySha256: "", sources: baselineSources },
    candidate: {
      revision: git("rev-parse", "HEAD").trim(),
      dirty: git("status", "--porcelain").length > 0,
      binarySha256: "",
      sources: candidateSources,
    },
    harnessSha256: hash(harness),
    assets,
    rustc: toolVersion("rustc"),
    cargo: toolVersion("cargo"),
    bun: Bun.version,
  };
  await writeJson(resolve(output, "build.json"), description);
  description.baseline.binarySha256 = await buildBinary("baseline", baseline);
  description.candidate.binarySha256 = await buildBinary("candidate", root);
  if (
    JSON.stringify(candidateSources) !==
      JSON.stringify(await snapshotSources(root, candidateFiles())) ||
    JSON.stringify(baselineSources) !==
      JSON.stringify(await snapshotSources(baseline, baselineFiles))
  )
    throw Error(
      "Source changed during the builds; repeat the build phase in a fresh output directory",
    );
  description.status = "built";
  await writeJson(resolve(output, "build.json"), description);
  await writeJson(resolve(output, "source-hashes.json"), {
    baseline: baselineSources,
    candidate: candidateSources,
  });
  await verifyBundle(description);
  console.log(`Saved both binaries and one shared asset set: ${output}`);
  console.log(
    `Run after other compilation has finished: bun test/performance/source-run.ts --phase run --output ${JSON.stringify(output)}`,
  );
  return description;
}

async function verifyBundle(build: Build) {
  for (const label of ["baseline", "candidate"] as const) {
    if (
      hash(await Bun.file(resolve(output, "bundle", label, binaryName)).bytes()) !==
      build[label].binarySha256
    )
      throw Error(`Saved ${label} binary changed`);
    if (
      hash(await Bun.file(resolve(output, "source", label, harnessPath)).bytes()) !==
      build.harnessSha256
    )
      throw Error(`Saved ${label} harness differs`);
  }
  for (const [file, expected] of Object.entries(build.assets))
    if (hash(await Bun.file(resolve(output, "bundle", file)).bytes()) !== expected)
      throw Error(`Saved fixture asset changed: ${file}`);
}

function checkNoBuilds() {
  const result = Bun.spawnSync(["ps", "-axo", "pid=,comm="], { stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) throw Error(`Cannot check for concurrent builds: ${result.stderr}`);
  const names = new Set([
    "cargo",
    "rustc",
    "rustc-real",
    "ink-compiler",
    "clang",
    "clang++",
    "ld",
    "ld.lld",
    "ninja",
    "cmake",
    "gradle",
    "kotlinc",
  ]);
  const active = result.stdout
    .toString()
    .split("\n")
    .filter((line) => {
      const command = line.trim().match(/^\d+\s+(.+)$/)?.[1];
      return command && names.has(basename(command));
    });
  if (active.length)
    throw Error(`Compilation is active; do not time concurrently:\n${active.join("\n")}`);
}

type Sample = {
  elapsed_ns: number;
  apply_ns: number;
  commits: number;
  nodes_measured: number;
  full_rebuilds: number;
  incremental_rebuilds: number;
};
type Workload = {
  name: string;
  native: boolean;
  count: number;
  samples: Sample[];
  setup: { startup: Sample; populate: Sample };
  diagnostics: unknown[];
  max_host_nodes: number;
  window_rows: number;
  contracts: {
    before: { passed: boolean };
    after: { passed: boolean };
    native_row_taps: boolean;
    full_reorder_and_restore: boolean;
  };
};
type Round = { version: number; workloads: Workload[] };
type Run = { label: "baseline" | "candidate"; block: number; file: string; report: Round };

function distribution(values: number[]) {
  const sorted = [...values].sort((left, right) => left - right);
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
  return reports[0].workloads.map((workload, index) => {
    const perProcess = reports.map((report) => report.workloads[index].samples);
    const samples = perProcess.flat();
    const commits = samples.map((sample) => sample.commits);
    return {
      name: workload.name,
      elapsed: distribution(samples.map((sample) => sample.elapsed_ns)),
      apply: distribution(samples.map((sample) => sample.apply_ns)),
      process_median_ms: perProcess.map(
        (samples) => distribution(samples.map((sample) => sample.elapsed_ns)).median_ms,
      ),
      commits: {
        min: Math.min(...commits),
        max: Math.max(...commits),
        mean: commits.reduce((sum, count) => sum + count, 0) / commits.length,
      },
      mean_nodes_measured:
        samples.reduce((sum, sample) => sum + sample.nodes_measured, 0) / samples.length,
    };
  });
}

async function runMeasurements(build: Build) {
  if (build.version !== 1 || build.status !== "built")
    throw Error("A completed build phase is required");
  if (JSON.stringify(build.machine) !== JSON.stringify(machine))
    throw Error("Saved binaries were built on a different host");
  if (await Bun.file(resolve(output, "result.json")).exists())
    throw Error(
      "This output already contains results; preserve it and use a fresh build/output for another run",
    );
  const blocks = integer(values.blocks!, "Blocks", 1, 50);
  const samples = integer(values.samples!, "Samples", 2, 10000);
  const warmup = integer(values.warmup!, "Warm-up", 0, 10000);
  const timeoutSeconds = integer(values["timeout-seconds"]!, "Timeout seconds", 1, 7200);
  if (samples % 2 || warmup % 2) throw Error("Samples and warm-up must both be even");
  const counts = values.counts!.split(",").map((value) => Number(value));
  if (
    !counts.length ||
    counts.some((count) => ![1000, 5000, 100000].includes(count)) ||
    new Set(counts).size !== counts.length
  )
    throw Error("Use --counts 1000,5000 or a subset; 100000 is exploratory only");
  if (counts.includes(100000))
    console.warn(
      "100,000 rows are exploratory: the array baseline can exceed the 1 MiB commit input limit, so this size cannot provide the main comparison.",
    );
  const expectedNames = counts.flatMap((count) => [
    `source-edit-${count}`,
    `source-replace-${count}`,
  ]);
  const runs: Run[] = [];
  const executionOrder: string[] = [];
  const startedAt = new Date().toISOString();
  const start = performance.now();
  await verifyBundle(build);
  try {
    for (let block = 0; block < blocks; block++) {
      const order =
        block % 2 === 0
          ? (["baseline", "candidate", "candidate", "baseline"] as const)
          : (["candidate", "baseline", "baseline", "candidate"] as const);
      for (const label of order) {
        checkNoBuilds();
        await verifyBundle(build);
        const name = `${String(runs.length).padStart(2, "0")}-${label}`;
        executionOrder.push(name);
        console.log(
          `Block ${block + 1}/${blocks}: ${name} (${samples} samples, ${warmup} warm-up)`,
        );
        const child = Bun.spawn(
          [
            resolve(output, "bundle", label, binaryName),
            String(samples),
            String(warmup),
            counts.join(","),
          ],
          {
            cwd: output,
            stdout: "pipe",
            stderr: "pipe",
            env: {
              ...process.env,
              INK_PERF_ASSETS: resolve(output, "bundle/assets"),
              INK_SOURCE_EXPECT_NATIVE: String(label === "candidate"),
              INK_SOURCE_RENDER_DIR: values.render ? resolve(output, "renders", name) : "",
            },
          },
        );
        let timedOut = false;
        const timeout = setTimeout(() => {
          timedOut = true;
          child.kill("SIGKILL");
        }, timeoutSeconds * 1000);
        const [code, stdout, stderr] = await Promise.all([
          child.exited,
          new Response(child.stdout).text(),
          new Response(child.stderr).text(),
        ]);
        clearTimeout(timeout);
        await Bun.write(resolve(output, `${name}.json`), stdout);
        await Bun.write(resolve(output, `${name}.log`), stderr);
        checkNoBuilds();
        if (code !== 0 || timedOut)
          throw Error(
            `${name} ${timedOut ? "timed out" : "failed"}; see ${output}/${name}.log\n${stderr}`,
          );
        const report: Round = JSON.parse(stdout);
        if (
          report.version !== 1 ||
          JSON.stringify(report.workloads.map((workload) => workload.name)) !==
            JSON.stringify(expectedNames)
        )
          throw Error(`${name}: unexpected workload names or version`);
        for (const workload of report.workloads) {
          if (workload.native !== (label === "candidate"))
            throw Error(`${name}: wrong fixture feature route`);
          if (
            workload.samples.length !== samples ||
            workload.samples.some(
              (sample) =>
                !Number.isFinite(sample.elapsed_ns) ||
                sample.elapsed_ns <= 0 ||
                !Number.isFinite(sample.apply_ns) ||
                sample.apply_ns < 0 ||
                sample.apply_ns > sample.elapsed_ns ||
                !Number.isInteger(sample.commits) ||
                sample.commits < 1,
            )
          )
            throw Error(`${name}: invalid samples for ${workload.name}`);
          if (
            !workload.contracts.before.passed ||
            !workload.contracts.after.passed ||
            !workload.contracts.native_row_taps ||
            !workload.contracts.full_reorder_and_restore ||
            workload.max_host_nodes >= 500 ||
            workload.window_rows < 1 ||
            workload.window_rows >= 100
          )
            throw Error(
              `${name}: correctness or bounded-window contract failed for ${workload.name}`,
            );
        }
        runs.push({ label, block, file: `${name}.json`, report });
      }
    }
    await verifyBundle(build);
  } catch (error) {
    await writeJson(resolve(output, "result.json"), {
      version: 1,
      status: "failed",
      error: String(error),
      startedAt,
      executionOrder,
      completed_runs: runs.map(({ label, block, file }) => ({ label, block, file })),
    });
    throw error;
  }
  const baseline = summarise(
    runs.filter((run) => run.label === "baseline").map((run) => run.report),
  );
  const candidate = summarise(
    runs.filter((run) => run.label === "candidate").map((run) => run.report),
  );
  const comparison = candidate.map((current, index) => ({
    name: current.name,
    median_change_percent:
      (current.elapsed.median_ms / baseline[index].elapsed.median_ms - 1) * 100,
    p95_change_percent: (current.elapsed.p95_ms / baseline[index].elapsed.p95_ms - 1) * 100,
    apply_median_change_percent:
      (current.apply.median_ms / baseline[index].apply.median_ms - 1) * 100,
    block_median_change_percent: Array.from({ length: blocks }, (_, block) => {
      const median = (label: string) =>
        distribution(
          runs
            .filter((run) => run.label === label && run.block === block)
            .flatMap((run) =>
              run.report.workloads[index].samples.map((sample) => sample.elapsed_ns),
            ),
        ).median_ms;
      return (median("candidate") / median("baseline") - 1) * 100;
    }),
  }));
  await writeJson(resolve(output, "result.json"), {
    version: 1,
    status: "passed",
    build: "build.json",
    machine,
    blocks,
    samples,
    warmup,
    counts,
    fresh_processes: runs.length,
    startedAt,
    completedAt: new Date().toISOString(),
    executionSeconds: (performance.now() - start) / 1000,
    executionOrder,
    raw_files: runs.map(({ label, block, file }) => ({ label, block, file })),
    baseline,
    candidate,
    comparison,
    timing:
      "dispatch through native scene readiness for both subtitle and unread count; no GPU or Android presentation",
    diagnostic_bytes:
      "separate untimed probes serialise protocol diagnostics; these are not measured transport bytes",
  });
  const rows = candidate.map((current, index) => {
    const previous = baseline[index];
    return `| ${current.name} | ${previous.elapsed.median_ms.toFixed(3)} | ${current.elapsed.median_ms.toFixed(3)} | ${comparison[index].median_change_percent.toFixed(1)} | ${current.elapsed.p95_ms.toFixed(3)} | ${previous.apply.median_ms.toFixed(3)} / ${current.apply.median_ms.toFixed(3)} | ${previous.commits.mean.toFixed(1)} / ${current.commits.mean.toFixed(1)} |`;
  });
  const report = `# Source performance

${runs.length} fresh processes in ${blocks} alternating ABBA blocks, with ${warmup} warm-up operations and ${samples} samples per workload/process. Release builds on ${machine.cpu} (${machine.platform}/${machine.arch}). Both runtimes use the same saved app.js and icon bytes; the baseline selects arrays and the candidate selects native sources.

| Workload | Baseline median ms | Candidate median ms | Change % | Candidate p95 ms | Apply median ms, baseline / candidate | Mean commits, baseline / candidate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
${rows.join("\n")}

Negative changes mean faster. Raw elapsed/apply nanoseconds, commit counts and native layout metrics are saved per process. Result metadata includes each block's median change; there is no automatic speed threshold.

Timing starts at command dispatch and ends when the changed row subtitle and correct unread count are both applied to a ready native scene. Apply time is the sum of ReactTree.apply calls until readiness. Startup and populate observations are saved separately from steady-state samples. Detailed scene assertions, memory inspection and byte serialisation run outside the measured samples. Diagnostic byte counts describe serialised protocol diagnostics, including metadata for native handles; they do not measure wire traffic or total retained source memory.

Correctness probes use native hit testing and route_native_message before and after edits, a full dataset reversal/restoration and replacement. Native-source contracts check retained snapshots, rejected transaction atomicity, no-op subscriber silence, multiple filter fields, missing versus null values, filtered replacement order and snapshot readability after release through the fixture. Host nodes and materialised row windows remain bounded. Existing scene checks cover regular array compatibility. Main comparisons use 1,000 and 5,000 rows: 10,000 rows exceed the array baseline's 1 MiB commit input limit on Android, and any explicitly requested 100,000-row workload is exploratory rather than a meaningful main comparison.

Asset preparation, file reads, Android process launch, GPU work and presentation are excluded from steady-state timing. Optional renders run after the final timed workload. Keep the host free of other builds throughout the run; the runner checks for active compilation before and after each process. Source snapshots, hashes, dirty-tree changes, revisions, binaries and shared assets are saved alongside this report.
`;
  await Bun.write(resolve(output, "report.md"), report);
  console.log(`Results: ${output}/result.json\nReport: ${output}/report.md`);
}

console.log(`Source benchmark evidence: ${output}`);
let build: Build;
if (values.phase === "build" || values.phase === "all") build = await prepareBuild();
else build = await Bun.file(resolve(output, "build.json")).json();
if (values.phase === "run" || values.phase === "all") await runMeasurements(build);
