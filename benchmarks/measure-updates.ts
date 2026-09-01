const adb = process.env.ADB ??
  `${process.env.HOME}/Library/Android/sdk/platform-tools/adb`;
const device = process.env.BENCHMARK_DEVICE;
const adbCommand = device ? [adb, "-s", device] : [adb];
const output = process.env.BENCHMARK_OUTPUT ??
  "benchmarks/results/ink-updates.json";
const rounds = Number(process.env.UPDATE_BENCHMARK_ROUNDS ?? 25);
const routeExtra = "com.vandam.ink.notification.HREF";

type Variant = {
  name: string;
  packageName: string;
  apk: string;
  revision: string;
  instrumented: boolean;
  expectedSha256?: string;
  component: string;
  bytes: number;
  sha256: string;
};

type VariantInput = Omit<Variant, "component" | "bytes" | "sha256" | "instrumented"> & {
  instrumented?: boolean;
};

const defaultPackageName = "com.vandam.benchmark.ink.updates";
const variantInputs: VariantInput[] = process.env.INK_UPDATE_VARIANTS
  ? JSON.parse(process.env.INK_UPDATE_VARIANTS)
  : [{
      name: "current",
      packageName: defaultPackageName,
      apk: process.env.INK_UPDATES_APK ??
        "benchmarks/apps/ink-updates/dist/ink-updates-benchmark-1.0.0-arm64.apk",
      revision: process.env.INK_BENCHMARK_REVISION ?? "",
      expectedSha256: process.env.INK_UPDATES_APK_SHA256,
    }];

type Scenario = {
  name: string;
  route: string;
  tapY: number;
};

type Sample = {
  cpuMs: number;
  startMs: number;
  phases?: Record<string, number>;
};

const scenarios: Scenario[] = [
  { name: "Text 7 → 8", route: "/text-same-width", tapY: 755 },
  { name: "Text 9 → 10", route: "/text-width-change", tapY: 755 },
  { name: "Toggle", route: "/toggle", tapY: 665 },
  { name: "Structure", route: "/structure", tapY: 755 },
  { name: "List append", route: "/list-append", tapY: 190 },
  { name: "Three-state burst", route: "/burst", tapY: 755 },
  { name: "Emoji cold path", route: "/emoji", tapY: 755 },
];

function run(command: string[], quiet = false): string {
  const result = Bun.spawnSync(command, { stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) {
    throw new Error(`${command.join(" ")}\n${result.stderr.toString()}`);
  }
  const stdout = result.stdout.toString().replaceAll("\r", "");
  if (!quiet && stdout.trim()) console.log(stdout.trim());
  return stdout;
}

function shell(command: string): string {
  return run([...adbCommand, "shell", command], true);
}

function sleep(milliseconds: number) {
  Bun.sleepSync(milliseconds);
}

function percentile(values: number[], fraction: number): number {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.ceil(sorted.length * fraction) - 1] ?? 0;
}

function mean(values: number[]): number {
  return values.reduce((total, value) => total + value, 0) / values.length;
}

function thermalStatus(): number {
  return Number(
    shell("dumpsys thermalservice").match(/Thermal Status:\s+(\d+)/)?.[1] ?? -1,
  );
}

function processCpuNanoseconds(variant: Variant): number {
  const pid = shell(`pidof -s ${variant.packageName}`).trim();
  if (!/^\d+$/.test(pid)) {
    throw new Error(`Could not find ${variant.packageName}`);
  }
  const value = shell(
    `awk '{ total += $1 } END { printf "%.0f", total }' /proc/${pid}/task/*/schedstat`,
  ).trim();
  const nanoseconds = Number(value);
  if (!Number.isFinite(nanoseconds)) {
    throw new Error(`Could not read CPU time for ${variant.packageName}`);
  }
  return nanoseconds;
}

function performancePhases(): { revision: string; phases: Record<string, number> } | undefined {
  const lines = shell("logcat -d -s Ink:I '*:S'")
    .split("\n")
    .filter((line) => line.includes("Perf revision="));
  const line = lines.findLast((candidate) => {
    const update = candidate.match(/update_ns=(\d+)/)?.[1];
    return update !== undefined && Number(update) > 0;
  });
  if (!line) return undefined;
  const revision = line.match(/revision=([^\s]+)/)?.[1];
  if (!revision) return undefined;
  return {
    revision,
    phases: Object.fromEntries(
      [...line.matchAll(/([a-z_]+)=(\d+)/g)].map((match) => [
        match[1],
        Number(match[2]),
      ]),
    ),
  };
}

function waitForPerformancePhases() {
  const deadline = performance.now() + 2_000;
  while (performance.now() < deadline) {
    const performance = performancePhases();
    if (performance) return performance;
    sleep(25);
  }
  throw new Error("The benchmark tap produced no instrumented update frame");
}

function screenshotHash(): string {
  const path = "/data/local/tmp/ink-update-benchmark.png";
  shell(`screencap -p ${path}`);
  return shell(`sha256sum ${path}`).split(/\s+/)[0];
}

function waitForVisualChange(previous: string) {
  const deadline = performance.now() + 2_000;
  while (performance.now() < deadline) {
    if (screenshotHash() !== previous) return;
    sleep(25);
  }
  throw new Error("The benchmark tap did not produce a visible state change");
}

function start(variant: Variant, route: string): number {
  for (const candidate of variants) {
    shell(`am force-stop ${candidate.packageName}`);
  }
  const result = shell(
    `am start -W -n ${variant.component} --es ${routeExtra} ${route}`,
  );
  const match = result.match(/^(?:TotalTime|WaitTime):\s+(\d+)/m);
  if (!match) throw new Error(`No launch time for ${route}:\n${result}`);
  return Number(match[1]);
}

if (!Number.isInteger(rounds) || rounds < 1) {
  throw new Error("UPDATE_BENCHMARK_ROUNDS must be a positive integer");
}
if (!Array.isArray(variantInputs) || variantInputs.length === 0) {
  throw new Error("INK_UPDATE_VARIANTS must contain at least one variant");
}
const variants = variantInputs.map((variant): Variant => {
  if (!variant.name || !variant.packageName || !variant.apk || !variant.revision) {
    throw new Error("Each update benchmark variant needs a name, package, APK, and revision");
  }
  const bytes = Bun.file(variant.apk).size;
  if (!bytes) throw new Error(`Could not find ${variant.apk}`);
  const sha256 = run(["shasum", "-a", "256", variant.apk], true).split(/\s+/)[0];
  if (variant.expectedSha256 && sha256 !== variant.expectedSha256) {
    throw new Error(`${variant.name} APK SHA-256 is ${sha256}; expected ${variant.expectedSha256}`);
  }
  return {
    ...variant,
    instrumented: variant.instrumented ?? true,
    component: `${variant.packageName}/com.vandam.ink.MainActivity`,
    bytes,
    sha256,
  };
});
if (new Set(variants.flatMap((variant) => [variant.name, variant.packageName])).size !== variants.length * 2) {
  throw new Error("Update benchmark variant names and package names must be unique");
}

run([...adbCommand, "wait-for-device"], true);
const originalStayOn = shell("settings get global stay_on_while_plugged_in").trim();
let restorePowerSetting = true;
function restorePower() {
  if (!restorePowerSetting) return;
  restorePowerSetting = false;
  Bun.spawnSync([
    ...adbCommand,
    "shell",
    "settings",
    "put",
    "global",
    "stay_on_while_plugged_in",
    originalStayOn,
  ]);
}
process.on("exit", restorePower);
shell("settings put global stay_on_while_plugged_in 7");
for (const variant of variants) {
  shell(`am force-stop ${variant.packageName}`);
}
shell("input keyevent 224");
sleep(300);
shell("input swipe 540 1150 540 300 300");
sleep(300);
if (!shell("dumpsys power").includes("mWakefulness=Awake")) {
  throw new Error("Device did not remain awake after the unlock gesture");
}
const startingThermalStatus = thermalStatus();
if (startingThermalStatus !== 0) {
  throw new Error(
    `Device thermal status is ${startingThermalStatus}; expected 0`,
  );
}
for (const variant of variants) {
  run([...adbCommand, "install", "-r", variant.apk], true);
}

const samplesByVariant = new Map(
  variants.map((variant) => [
    variant.name,
    new Map(scenarios.map((scenario) => [scenario.name, [] as Sample[]])),
  ]),
);
for (let round = 0; round < rounds; round += 1) {
  const direction = round % 2 === 0 ? scenarios : [...scenarios].reverse();
  const offset = Math.floor(round / 2) % scenarios.length;
  const order = [...direction.slice(offset), ...direction.slice(0, offset)];
  for (let scenarioIndex = 0; scenarioIndex < order.length; scenarioIndex += 1) {
    const scenario = order[scenarioIndex];
    const variantOffset = (round + scenarioIndex) % variants.length;
    const variantOrder = [
      ...variants.slice(variantOffset),
      ...variants.slice(0, variantOffset),
    ];
    for (const variant of variantOrder) {
      const startMs = start(variant, scenario.route);
      sleep(500);
      const screenshotBefore = variant.instrumented ? undefined : screenshotHash();
      shell("logcat -c");
      const cpuBefore = processCpuNanoseconds(variant);
      shell(`input tap 540 ${scenario.tapY}`);
      const performance = variant.instrumented ? waitForPerformancePhases() : undefined;
      if (screenshotBefore) waitForVisualChange(screenshotBefore);
      const cpuMs = (processCpuNanoseconds(variant) - cpuBefore) / 1_000_000;
      if (performance && performance.revision !== variant.revision) {
        throw new Error(
          `${variant.name} APK reports revision ${performance.revision}; expected ${variant.revision}`,
        );
      }
      if (performance &&
        (performance.phases.full_rebuilds ?? 0) +
          (performance.phases.incremental_rebuilds ?? 0) ===
        0
      ) {
        throw new Error(`${variant.name} ${scenario.name} did not rebuild after its benchmark tap`);
      }
      samplesByVariant.get(variant.name)?.get(scenario.name)?.push({
        cpuMs,
        startMs,
        ...(performance ? { phases: performance.phases } : {}),
      });
    }
  }
  console.log(`Update round ${round + 1}/${rounds}`);
}

const results = variants.map((variant) => ({
  name: variant.name,
  revision: variant.revision,
  instrumented: variant.instrumented,
  apk: {
    path: variant.apk,
    bytes: variant.bytes,
    sha256: variant.sha256,
  },
  scenarios: scenarios.map((scenario) => {
    const samples = samplesByVariant.get(variant.name)?.get(scenario.name) ?? [];
    const cpu = samples.map((sample) => sample.cpuMs);
    const starts = samples.map((sample) => sample.startMs);
    const phaseNames = new Set(
      samples.flatMap((sample) => Object.keys(sample.phases ?? {})),
    );
    const summary = {
      name: scenario.name,
      route: scenario.route,
      samples,
      cpuMeanMs: mean(cpu),
      cpuMedianMs: percentile(cpu, 0.5),
      cpuP95Ms: percentile(cpu, 0.95),
      startMedianMs: percentile(starts, 0.5),
      phases: Object.fromEntries(
        [...phaseNames].map((name) => {
          const values = samples.flatMap((sample) =>
            sample.phases?.[name] === undefined ? [] : [sample.phases[name]],
          );
          return [
            name,
            {
              mean: mean(values),
              median: percentile(values, 0.5),
              p95: percentile(values, 0.95),
            },
          ];
        }),
      ),
    };
    console.log(
      `${variant.name} ${scenario.name}: ${summary.cpuMeanMs.toFixed(2)} ms mean CPU, ` +
        `${summary.cpuP95Ms.toFixed(2)} ms P95`,
    );
    return summary;
  }),
}));

const payload = {
  timestamp: new Date().toISOString(),
  harnessRevision: run(["git", "rev-parse", "HEAD"], true).trim(),
  environment: {
    device: device ?? "default",
    model: shell("getprop ro.product.model").trim(),
    sdk: Number(shell("getprop ro.build.version.sdk").trim()),
    thermalStatusStart: startingThermalStatus,
    thermalStatusEnd: thermalStatus(),
  },
  rounds,
  results,
};

await Bun.write(output, `${JSON.stringify(payload, null, 2)}\n`);
restorePower();
console.log(`Wrote ${output}`);
