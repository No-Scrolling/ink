import { parseArgs } from "node:util";
import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";
import { arch, cpus, platform, release } from "node:os";

const { values } = parseArgs({
  options: {
    baseline: { type: "string" },
    candidate: { type: "string" },
    output: { type: "string" },
    samples: { type: "string", default: "64" },
    warmup: { type: "string", default: "12" },
    controls: { type: "boolean", default: false },
    help: { type: "boolean" },
  },
});
if (values.help || !values.baseline || !values.candidate || !values.output) {
  console.log(
    "Usage: bun test/performance/automatic-run.ts --baseline BUNDLE --candidate BUNDLE --output DIRECTORY [--samples 64 --warmup 12 --controls]",
  );
  console.log(
    "Build both versions first. Each bundle contains ink-automatic-performance and assets/automatic. --controls also requires ink-performance and its six fixture assets. Runs four fresh processes in baseline/candidate/candidate/baseline order; never builds during timing.",
  );
  process.exit(values.help ? 0 : 2);
}
const samples = Number(values.samples),
  warmup = Number(values.warmup);
if (!Number.isInteger(samples) || samples < 2 || !Number.isInteger(warmup) || warmup < 0)
  throw Error("Samples must be at least two; warm-up must be non-negative");
const output = resolve(values.output);
await mkdir(output, { recursive: true });
const bundles = { before: resolve(values.baseline), after: resolve(values.candidate) };
const hash = async (path: string) =>
  new Bun.CryptoHasher("sha256").update(await Bun.file(path).bytes()).digest("hex");
const manifest: Record<string, unknown> = {};
for (const [variant, bundle] of Object.entries(bundles)) {
  manifest[variant] = {
    bundle,
    binary: await hash(resolve(bundle, "ink-automatic-performance")),
    javascript: await hash(resolve(bundle, "assets/automatic/app.js")),
  };
}
function noBuilds() {
  const processes = Bun.spawnSync(["ps", "-Ao", "comm"], { stdout: "pipe" })
    .stdout.toString()
    .split("\n")
    .filter((line) => /\/(?:rustc|cargo|clang|clang\+\+)$/.test(line));
  if (processes.length) throw Error("Finish compilation before measuring: " + processes.join(", "));
}
type Suite = "automatic" | "controls";
type Sample = { elapsed_ns: number };
type Input = {
  arrival_to_ack_ns: number;
  dispatch_to_ack_ns: number;
  before_list_ready: boolean;
};
type Job = { list_ready_ns: number; inputs: Input[] };
type Workload =
  | { name: string; samples: Sample[]; jobs?: never }
  | { name: string; jobs: Job[]; samples?: never };
type Report = { workloads: Workload[] };
const order = ["before", "after", "after", "before"] as const;
const suites: Suite[] = values.controls ? ["automatic", "controls"] : ["automatic"];
const runs: { variant: (typeof order)[number]; block: number; suite: Suite; report: Report }[] = [];
for (const [block, variant] of order.entries()) {
  noBuilds();
  const bundle = bundles[variant];
  for (const suite of suites) {
    const args =
      suite === "automatic" ? [String(samples), String(warmup), "1000,5000"] : ["40", "8"];
    const binary = suite === "automatic" ? "ink-automatic-performance" : "ink-performance";
    const child = Bun.spawn([resolve(bundle, binary), ...args], {
      env: {
        ...process.env,
        INK_AUTOMATIC_ASSETS: resolve(bundle, "assets/automatic"),
        INK_PERF_ASSETS: resolve(bundle, "assets"),
      },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [code, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    await Bun.write(resolve(output, `${block}-${variant}-${suite}.json`), stdout);
    await Bun.write(resolve(output, `${block}-${variant}-${suite}.log`), stderr);
    if (code) throw Error(`${variant} ${suite} failed: ${stderr}`);
    const report: Report = JSON.parse(stdout);
    runs.push({ variant, block, suite, report });
  }
  noBuilds();
  console.log(`Paired block ${block + 1}/4: ${variant} passed`);
}
function stats(values: number[]) {
  const sorted = values.toSorted((a, b) => a - b);
  const q = (p: number) => sorted[Math.ceil(p * sorted.length) - 1] / 1e6;
  return { n: sorted.length, median_ms: q(0.5), p95_ms: q(0.95) };
}
const summary: Record<string, unknown> = {};
for (const suite of suites) {
  const selectedRuns = runs.filter((run) => run.suite === suite);
  const names = selectedRuns[0].report.workloads.map((work) => work.name);
  summary[suite] = Object.fromEntries(
    names.map((name: string) => [
      name,
      Object.fromEntries(
        (["before", "after"] as const).map((variant) => {
          const selected = selectedRuns
            .filter((run) => run.variant === variant)
            .map((run) => {
              const work = run.report.workloads.find((work) => work.name === name);
              if (!work) throw Error(`Missing workload: ${name}`);
              return work;
            });
          if (selected[0].jobs) {
            const jobs = selected.flatMap((work) => {
              if (!work.jobs) throw Error(`Missing typing jobs: ${name}`);
              return work.jobs;
            });
            const inputs = jobs.flatMap((job) => job.inputs);
            return [
              variant,
              {
                arrival_to_ack: stats(inputs.map((input) => input.arrival_to_ack_ns)),
                dispatch_to_ack: stats(inputs.map((input) => input.dispatch_to_ack_ns)),
                list_ready: stats(jobs.map((job) => job.list_ready_ns)),
                acknowledgements_before_list_ready: inputs.filter(
                  (input) => input.before_list_ready,
                ).length,
              },
            ];
          }
          return [
            variant,
            stats(
              selected.flatMap((work) => {
                if (!work.samples) throw Error(`Missing timing samples: ${name}`);
                return work.samples.map((sample) => sample.elapsed_ns);
              }),
            ),
          ];
        }),
      ),
    ]),
  );
}
await Bun.write(
  resolve(output, "summary.json"),
  JSON.stringify(
    {
      createdAt: new Date().toISOString(),
      machine: { platform: platform(), arch: arch(), os: release(), cpu: cpus()[0]?.model },
      order,
      samples,
      warmup,
      bundles: manifest,
      summary,
    },
    null,
    2,
  ) + "\n",
);
console.log("Saved paired raw results and summary: " + output);
