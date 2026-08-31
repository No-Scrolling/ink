const adb = process.env.ADB ?? `${process.env.HOME}/Library/Android/sdk/platform-tools/adb`;
const adbCommand = process.env.BENCHMARK_DEVICE
  ? [adb, "-s", process.env.BENCHMARK_DEVICE]
  : [adb];
const output = process.env.BENCHMARK_OUTPUT ?? "benchmarks/results/raw.json";
let clockTicksPerSecond = 100;

type App = {
  key: "ink" | "expo" | "light-sdk";
  stack: "Ink" | "Expo" | "Light SDK";
  packageName: string;
  component: string;
  apk: string;
} & (
  | { scenario: "Counter"; counterTapY: number }
  | { scenario: "Scroll" }
);

type MemorySample = {
  pssKb: number;
  rssKb: number;
  threads: number;
};

type WorkloadSample = {
  cpuMs: number;
  elapsedMs: number;
  cpuPercent: number;
  frames: number;
  droppedFrames: number;
  averageFps: number;
  presentP95Ms: number;
  compositorP95Ms: number;
  pssKb: number;
  rssKb: number;
};

type ContinuousScrollSample = ReturnType<typeof surfaceStats>;

const stackKeys = ["ink", "expo", "light-sdk"] as const;
const scenarioKeys = ["Counter", "Scroll"] as const;
const selectedStacks = new Set(
  (process.env.BENCHMARK_STACKS ?? stackKeys.join(","))
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean),
);
const selectedScenarios = new Set(
  (process.env.BENCHMARK_SCENARIOS ?? scenarioKeys.join(","))
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean),
);
for (const stack of selectedStacks) {
  if (!stackKeys.includes(stack as (typeof stackKeys)[number])) {
    throw new Error(`Unknown benchmark stack: ${stack}`);
  }
}
for (const scenario of selectedScenarios) {
  if (!scenarioKeys.includes(scenario as (typeof scenarioKeys)[number])) {
    throw new Error(`Unknown benchmark scenario: ${scenario}`);
  }
}

const expoCounter = process.env.EXPO_COUNTER_APK ?? "";
const expoScroll = process.env.EXPO_SCROLL_APK ?? "";
const inkCounter = process.env.INK_COUNTER_APK ??
  "benchmarks/apps/ink-counter/dist/ink-counter-benchmark-1.0.0-arm64.apk";
const inkScroll = process.env.INK_SCROLL_APK ??
  "benchmarks/apps/ink-scroll/dist/ink-scroll-benchmark-1.0.0-arm64.apk";
if (selectedStacks.has("expo") && (!expoCounter || !expoScroll)) {
  throw new Error("Expo benchmarks require EXPO_COUNTER_APK and EXPO_SCROLL_APK");
}

const apps: App[] = [
  {
    key: "ink",
    stack: "Ink",
    scenario: "Counter",
    packageName: "com.vandam.benchmark.ink.counter",
    component: "com.vandam.benchmark.ink.counter/com.vandam.ink.MainActivity",
    apk: inkCounter,
    counterTapY: 760,
  },
  {
    key: "expo",
    stack: "Expo",
    scenario: "Counter",
    packageName: "com.vandam.benchmark.expo.counter",
    component: "com.vandam.benchmark.expo.counter/.MainActivity",
    apk: expoCounter,
    counterTapY: 810,
  },
  {
    key: "light-sdk",
    stack: "Light SDK",
    scenario: "Counter",
    packageName: "com.vandam.benchmark.lightsdk.counter",
    component: "com.vandam.benchmark.lightsdk.counter/com.thelightphone.sdk.LightActivity",
    apk: "benchmarks/apps/light-sdk-counter/build/outputs/apk/release/benchmark-counter-release.apk",
    counterTapY: 760,
  },
  {
    key: "light-sdk",
    stack: "Light SDK",
    scenario: "Scroll",
    packageName: "com.vandam.benchmark.lightsdk.scroll",
    component: "com.vandam.benchmark.lightsdk.scroll/com.thelightphone.sdk.LightActivity",
    apk: "benchmarks/apps/light-sdk-scroll/build/outputs/apk/release/benchmark-scroll-release.apk",
  },
  {
    key: "expo",
    stack: "Expo",
    scenario: "Scroll",
    packageName: "com.vandam.benchmark.expo.scroll",
    component: "com.vandam.benchmark.expo.scroll/.MainActivity",
    apk: expoScroll,
  },
  {
    key: "ink",
    stack: "Ink",
    scenario: "Scroll",
    packageName: "com.vandam.benchmark.ink.scroll",
    component: "com.vandam.benchmark.ink.scroll/com.vandam.ink.MainActivity",
    apk: inkScroll,
  },
].filter((app) => selectedStacks.has(app.key) && selectedScenarios.has(app.scenario));

if (apps.length === 0) {
  throw new Error("BENCHMARK_STACKS selected no benchmark apps");
}

function run(command: string[], quiet = false): string {
  const result = Bun.spawnSync(command, { stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) {
    throw new Error(`${command.join(" ")}\n${result.stderr.toString()}`);
  }
  const stdout = result.stdout.toString().replaceAll("\r", "");
  if (!quiet && stdout.trim()) console.log(stdout.trim());
  return stdout;
}

function shell(command: string, quiet = true): string {
  return run([...adbCommand, "shell", command], quiet);
}

function sleep(milliseconds: number) {
  Bun.sleepSync(milliseconds);
}

function percentile(values: number[], fraction: number): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.ceil(sorted.length * fraction) - 1] ?? sorted.at(-1) ?? 0;
}

function histogramPercentile(line: string | undefined, fraction: number): number {
  if (!line) return 0;
  const buckets = [...line.matchAll(/(\d+)ms=(\d+)/g)].map((match) => ({
    milliseconds: Number(match[1]),
    count: Number(match[2]),
  }));
  const total = buckets.reduce((sum, bucket) => sum + bucket.count, 0);
  if (total === 0) return 0;
  const target = Math.ceil(total * fraction);
  let seen = 0;
  for (const bucket of buckets) {
    seen += bucket.count;
    if (seen >= target) return bucket.milliseconds;
  }
  return buckets.at(-1)?.milliseconds ?? 0;
}

function histogramCountAfter(line: string | undefined, milliseconds: number): number {
  if (!line) return 0;
  return [...line.matchAll(/(\d+)ms=(\d+)/g)].reduce(
    (count, match) => count + (Number(match[1]) > milliseconds ? Number(match[2]) : 0),
    0,
  );
}

function apkBreakdown(apk: string) {
  const bytes = Bun.file(apk).size;
  const listing = run(["unzip", "-l", apk], true);
  const groups = { dex: 0, native: 0, resources: 0, assets: 0, other: 0 };
  for (const line of listing.split("\n")) {
    const match = line.match(/^\s*(\d+)\s+\S+\s+\S+\s+(.+)$/);
    if (!match) continue;
    const size = Number(match[1]);
    const name = match[2];
    if (name.endsWith(".dex")) groups.dex += size;
    else if (name.startsWith("lib/")) groups.native += size;
    else if (name.startsWith("res/") || name === "resources.arsc") groups.resources += size;
    else if (name.startsWith("assets/")) groups.assets += size;
    else groups.other += size;
  }
  return { bytes, uncompressedBytes: Object.values(groups).reduce((a, b) => a + b, 0), ...groups };
}

function start(app: App): number {
  for (const candidate of apps) {
    shell(`am force-stop ${candidate.packageName}`);
  }
  const result = shell(`am start -W -n ${app.component}`);
  const match = result.match(/^(?:TotalTime|WaitTime):\s+(\d+)/m);
  if (!match) throw new Error(`No launch time for ${app.packageName}:\n${result}`);
  return Number(match[1]);
}

function memory(app: App): MemorySample {
  const result = shell(`dumpsys meminfo ${app.packageName}`);
  const pss = result.match(/TOTAL PSS:\s+(\d+)/);
  const rss = result.match(/TOTAL RSS:\s+(\d+)/);
  const pid = shell(`pidof -s ${app.packageName}`).trim();
  const threads = Number(shell(`ls /proc/${pid}/task | wc -l`).trim());
  if (!pss || !rss || !pid || !Number.isFinite(threads)) {
    throw new Error(`Could not read memory for ${app.packageName}`);
  }
  return { pssKb: Number(pss[1]), rssKb: Number(rss[1]), threads };
}

function cpuTicks(app: App): number {
  const pid = shell(`pidof -s ${app.packageName}`).trim();
  const stat = shell(`cat /proc/${pid}/stat`).trim();
  const end = stat.lastIndexOf(")");
  const fields = stat.slice(end + 2).split(/\s+/);
  return Number(fields[11]) + Number(fields[12]);
}

function thermalStatus(): number {
  return Number(
    shell("dumpsys thermalservice").match(/Thermal Status:\s+(\d+)/)?.[1] ?? -1,
  );
}

function surfaceStats(app: App) {
  const dump = shell("dumpsys SurfaceFlinger --timestats -dump");
  const blocks = dump.match(/displayRefreshRate =[\s\S]*?(?=\ndisplayRefreshRate =|$)/g) ?? [];
  const candidates = blocks
    .filter((block) => block.includes(app.packageName))
    .map((block) => ({ block, frames: Number(block.match(/totalFrames = (\d+)/)?.[1] ?? 0) }))
    .sort((a, b) => b.frames - a.frames);
  const block = candidates[0]?.block ?? "";
  const histogramAfter = (name: string) => block.match(new RegExp(`${name} histogram is as below:\\n([^\\n]+)`))?.[1];
  const presented = histogramAfter("present2present");
  return {
    frames: candidates[0]?.frames ?? 0,
    droppedFrames: Number(block.match(/droppedFrames = (\d+)/)?.[1] ?? 0),
    lateAcquireFrames: Number(block.match(/lateAcquireFrames = (\d+)/)?.[1] ?? 0),
    jankyFrames: Number(block.match(/jankyFrames = (\d+)/)?.[1] ?? 0),
    longPresentIntervals: histogramCountAfter(presented, 17),
    averageFps: Number(block.match(/averageFPS = ([\d.]+)/)?.[1] ?? 0),
    presentP95Ms: histogramPercentile(presented, 0.95),
    presentP99Ms: histogramPercentile(presented, 0.99),
    acquireP95Ms: histogramPercentile(histogramAfter("acquire2present"), 0.95),
    compositorP95Ms: histogramPercentile(histogramAfter("latch2present"), 0.95),
  };
}

function workload(app: App): WorkloadSample {
  start(app);
  sleep(2_000);
  shell("dumpsys SurfaceFlinger --timestats -clear");
  const before = cpuTicks(app);
  const started = performance.now();

  if (app.scenario === "Counter") {
    for (let index = 0; index < 100; index += 1) {
      shell(`input tap 540 ${app.counterTapY}`);
    }
  } else {
    for (let index = 0; index < 6; index += 1) {
      shell("input swipe 540 1050 540 180 350");
    }
    for (let index = 0; index < 6; index += 1) {
      shell("input swipe 540 180 540 1050 350");
    }
  }

  sleep(750);
  const elapsedMs = performance.now() - started;
  const cpuMs = ((cpuTicks(app) - before) * 1_000) / clockTicksPerSecond;
  return {
    cpuMs,
    elapsedMs,
    cpuPercent: (cpuMs / elapsedMs) * 100,
    ...surfaceStats(app),
    ...memory(app),
  };
}

function continuousScroll(app: App): ContinuousScrollSample {
  start(app);
  sleep(2_000);
  shell("dumpsys SurfaceFlinger --timestats -clear");
  shell("input swipe 540 1050 540 180 5000");
  return surfaceStats(app);
}

run([...adbCommand, "wait-for-device"], true);
clockTicksPerSecond = Number(shell("getconf CLK_TCK").trim());
if (!Number.isFinite(clockTicksPerSecond) || clockTicksPerSecond <= 0) {
  throw new Error("Could not read the device clock tick rate");
}
shell("settings put global window_animation_scale 0");
shell("settings put global transition_animation_scale 0");
shell("settings put global animator_duration_scale 0");
shell("dumpsys SurfaceFlinger --timestats -enable");
const thermalStatusStart = thermalStatus();
if (thermalStatusStart !== 0) {
  throw new Error(`Device thermal status is ${thermalStatusStart}; expected 0`);
}

for (const app of apps) {
  console.log(`Installing ${app.stack} ${app.scenario}`);
  run([...adbCommand, "install", "-r", app.apk], true);
  start(app);
  sleep(2_000);
}

const startup = Object.fromEntries(apps.map((app) => [app.packageName, [] as number[]]));
for (let round = 0; round < 15; round += 1) {
  const order = round % 2 === 0 ? apps : [...apps].reverse();
  for (const app of order) {
    startup[app.packageName].push(start(app));
    sleep(350);
  }
  console.log(`Startup round ${round + 1}/15`);
}

const idle = Object.fromEntries(apps.map((app) => [app.packageName, [] as MemorySample[]]));
for (let round = 0; round < 5; round += 1) {
  const order = round % 2 === 0 ? apps : [...apps].reverse();
  for (const app of order) {
    start(app);
    sleep(2_000);
    idle[app.packageName].push(memory(app));
  }
  console.log(`Idle-memory round ${round + 1}/5`);
}

const workloads = Object.fromEntries(apps.map((app) => [app.packageName, [] as WorkloadSample[]]));
for (let round = 0; round < 5; round += 1) {
  const order = round % 2 === 0 ? apps : [...apps].reverse();
  for (const app of order) {
    console.log(`Workload ${round + 1}/5: ${app.stack} ${app.scenario}`);
    workloads[app.packageName].push(workload(app));
  }
}

const continuousScrolls = Object.fromEntries(
  apps
    .filter((app) => app.scenario === "Scroll")
    .map((app) => [app.packageName, [] as ContinuousScrollSample[]]),
);
for (let round = 0; round < 3; round += 1) {
  const scrollApps = apps.filter((app) => app.scenario === "Scroll");
  const order = round % 2 === 0 ? scrollApps : [...scrollApps].reverse();
  for (const app of order) {
    console.log(`Continuous scroll ${round + 1}/3: ${app.stack}`);
    continuousScrolls[app.packageName].push(continuousScroll(app));
  }
}

const display = shell("dumpsys display");
const refreshRate = Number(display.match(/renderFrameRate\s+([\d.]+)/)?.[1]);
if (!Number.isFinite(refreshRate)) {
  throw new Error("Could not read the display refresh rate");
}

const environment = {
  capturedAt: new Date().toISOString(),
  device: shell("getprop ro.product.name").trim(),
  model: shell("getprop ro.product.model").trim(),
  android: shell("getprop ro.build.version.release").trim(),
  sdk: Number(shell("getprop ro.build.version.sdk").trim()),
  abi: shell("getprop ro.product.cpu.abi").trim(),
  resolution: shell("wm size").trim(),
  density: shell("wm density").trim(),
  refreshRate: Math.round(refreshRate),
  clockTicksPerSecond,
  thermalStatusStart,
  thermalStatusEnd: thermalStatus(),
};

const results = apps.map((app) => {
  const starts = startup[app.packageName];
  const memories = idle[app.packageName];
  const samples = workloads[app.packageName];
  const median = <K extends keyof WorkloadSample>(key: K) => percentile(samples.map((sample) => sample[key] as number), 0.5);
  const continuousSamples = continuousScrolls[app.packageName] ?? [];
  const continuousMedian = <K extends keyof ContinuousScrollSample>(key: K) =>
    percentile(continuousSamples.map((sample) => sample[key] as number), 0.5);
  return {
    ...app,
    apk: apkBreakdown(app.apk),
    startup: {
      samplesMs: starts,
      medianMs: percentile(starts, 0.5),
      p95Ms: percentile(starts, 0.95),
    },
    idleMemory: {
      samples: memories,
      medianPssKb: percentile(memories.map((sample) => sample.pssKb), 0.5),
      medianRssKb: percentile(memories.map((sample) => sample.rssKb), 0.5),
      medianThreads: percentile(memories.map((sample) => sample.threads), 0.5),
    },
    workload: {
      samples,
      medianCpuMs: median("cpuMs"),
      medianCpuPercent: median("cpuPercent"),
      medianFrames: median("frames"),
      medianDroppedFrames: median("droppedFrames"),
      medianAverageFps: median("averageFps"),
      medianPresentP95Ms: median("presentP95Ms"),
      medianCompositorP95Ms: median("compositorP95Ms"),
      medianPssKb: median("pssKb"),
      medianRssKb: median("rssKb"),
    },
    continuousScroll: app.scenario === "Scroll"
      ? {
          samples: continuousSamples,
          medianFrames: continuousMedian("frames"),
          medianDroppedFrames: continuousMedian("droppedFrames"),
          medianLateAcquireFrames: continuousMedian("lateAcquireFrames"),
          medianJankyFrames: continuousMedian("jankyFrames"),
          medianLongPresentIntervals: continuousMedian("longPresentIntervals"),
          medianPresentP95Ms: continuousMedian("presentP95Ms"),
          medianPresentP99Ms: continuousMedian("presentP99Ms"),
          medianAcquireP95Ms: continuousMedian("acquireP95Ms"),
        }
      : undefined,
  };
});

const protocol = {
  stacks: [...selectedStacks],
  scenarios: [...selectedScenarios],
  startupRuns: 15,
  idleRuns: 5,
  workloadRuns: 5,
  counterTaps: 100,
  scrollSwipesEachDirection: 6,
  swipeDurationMs: 350,
  continuousScrollRuns: 3,
  continuousScrollDurationMs: 5_000,
};
await Bun.write(
  output,
  `${JSON.stringify({ environment, protocol, results }, null, 2)}\n`,
);
console.log(`Wrote ${output}`);
