const adb = process.env.ADB ??
  `${process.env.HOME}/Library/Android/sdk/platform-tools/adb`;
const device = process.env.BENCHMARK_DEVICE;
const adbCommand = device ? [adb, "-s", device] : [adb];
const output = process.env.BENCHMARK_OUTPUT ??
  "benchmarks/results/ink-updates.json";
const rounds = Number(process.env.UPDATE_BENCHMARK_ROUNDS ?? 25);
const packageName = "com.vandam.benchmark.ink.updates";
const component = `${packageName}/com.vandam.ink.MainActivity`;
const apk = process.env.INK_UPDATES_APK ??
  "benchmarks/apps/ink-updates/dist/ink-updates-benchmark-1.0.0-arm64.apk";
const routeExtra = "com.vandam.ink.notification.HREF";

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

function processCpuNanoseconds(): number {
  const pid = shell(`pidof -s ${packageName}`).trim();
  if (!/^\d+$/.test(pid)) {
    throw new Error(`Could not find ${packageName}`);
  }
  const value = shell(
    `awk '{ total += $1 } END { printf "%.0f", total }' /proc/${pid}/task/*/schedstat`,
  ).trim();
  const nanoseconds = Number(value);
  if (!Number.isFinite(nanoseconds)) {
    throw new Error(`Could not read CPU time for ${packageName}`);
  }
  return nanoseconds;
}

function performancePhases(): Record<string, number> | undefined {
  const lines = shell("logcat -d -s Ink:I '*:S'")
    .split("\n")
    .filter((line) => line.includes("Perf update_ns="));
  const line = lines.at(-1);
  if (!line) return undefined;
  return Object.fromEntries(
    [...line.matchAll(/([a-z_]+)=(\d+)/g)].map((match) => [
      match[1],
      Number(match[2]),
    ]),
  );
}

function start(route: string): number {
  shell(`am force-stop ${packageName}`);
  const result = shell(
    `am start -W -n ${component} --es ${routeExtra} ${route}`,
  );
  const match = result.match(/^(?:TotalTime|WaitTime):\s+(\d+)/m);
  if (!match) throw new Error(`No launch time for ${route}:\n${result}`);
  return Number(match[1]);
}

if (!Number.isInteger(rounds) || rounds < 1) {
  throw new Error("UPDATE_BENCHMARK_ROUNDS must be a positive integer");
}
if (!Bun.file(apk).size) {
  throw new Error(`Could not find ${apk}`);
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
shell("input keyevent 224");
sleep(300);
shell("input swipe 540 1150 540 300 300");
sleep(300);
const startingThermalStatus = thermalStatus();
if (startingThermalStatus !== 0) {
  throw new Error(
    `Device thermal status is ${startingThermalStatus}; expected 0`,
  );
}
run([...adbCommand, "install", "-r", apk], true);

const results = [];
for (const scenario of scenarios) {
  const samples: Sample[] = [];
  for (let round = 0; round < rounds; round += 1) {
    const startMs = start(scenario.route);
    sleep(500);
    shell("logcat -c");
    const cpuBefore = processCpuNanoseconds();
    shell(`input tap 540 ${scenario.tapY}`);
    sleep(250);
    const cpuMs = (processCpuNanoseconds() - cpuBefore) / 1_000_000;
    const phases = performancePhases();
    samples.push({ cpuMs, startMs, ...(phases ? { phases } : {}) });
  }
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
  results.push(summary);
  console.log(
    `${scenario.name}: ${summary.cpuMeanMs.toFixed(2)} ms mean CPU, ` +
      `${summary.cpuP95Ms.toFixed(2)} ms P95`,
  );
}

const payload = {
  timestamp: new Date().toISOString(),
  revision: run(["git", "rev-parse", "HEAD"], true).trim(),
  environment: {
    device: device ?? "default",
    model: shell("getprop ro.product.model").trim(),
    sdk: Number(shell("getprop ro.build.version.sdk").trim()),
    thermalStatusStart: startingThermalStatus,
    thermalStatusEnd: thermalStatus(),
  },
  apk: {
    path: apk,
    bytes: Bun.file(apk).size,
  },
  rounds,
  results,
};

await Bun.write(output, `${JSON.stringify(payload, null, 2)}\n`);
restorePower();
console.log(`Wrote ${output}`);
