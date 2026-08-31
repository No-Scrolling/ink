const resultsPath = process.argv[2] ?? "benchmarks/results/ink.json";
const buildPath = process.argv[3];

type Limit = {
  name: string;
  actual: number;
  budget: number;
  direction: "maximum" | "minimum";
  unit: string;
};

type RuntimeResult = {
  stack: string;
  scenario: "Counter" | "Scroll";
  apk: { bytes: number };
  startup: { medianMs: number; p95Ms: number };
  idleMemory: {
    medianPssKb: number;
    medianRssKb: number;
    medianThreads: number;
  };
  workload: {
    medianCpuMs: number;
    medianAverageFps: number;
    medianPresentP95Ms: number;
  };
  continuousScroll?: {
    medianDroppedFrames: number;
    medianLateAcquireFrames: number;
    medianJankyFrames: number;
    medianLongPresentIntervals: number;
    medianPresentP95Ms: number;
    medianPresentP99Ms: number;
    medianAcquireP95Ms: number;
  };
};

type Results = {
  environment: {
    model: string;
    sdk: number;
    abi: string;
    resolution: string;
    refreshRate: number;
    thermalStatusStart: number;
    thermalStatusEnd: number;
  };
  results: RuntimeResult[];
};

type ScenarioBudget = {
  apkBytes: number;
  startupMedianMs: number;
  startupP95Ms: number;
  idlePssKb: number;
  idleRssKb: number;
  threads: number;
  workloadCpuMs: number;
  averageFps?: number;
  presentP95Ms?: number;
  continuousPresentP95Ms?: number;
  continuousPresentP99Ms?: number;
  continuousAcquireP95Ms?: number;
  continuousLongPresentIntervals?: number;
  continuousDroppedFrames?: number;
  continuousJankyFrames?: number;
  continuousLateAcquireFrames?: number;
};

type Budgets = {
  environment: {
    sdk: number;
    abi: string;
    width: number;
    height: number;
    refreshRate: number;
  };
  counter: ScenarioBudget;
  scroll: ScenarioBudget;
  build: {
    counterCleanSeconds: number;
    scrollCleanSeconds: number;
    scrollNoopSeconds: number;
  };
};

const results = await Bun.file(resultsPath).json() as Results;
const budgetsPath = process.env.INK_BENCHMARK_BUDGETS ??
  (results.environment.model === "TLP301"
    ? "benchmarks/budgets-lp3.json"
    : "benchmarks/budgets.json");
const budgets = await Bun.file(budgetsPath).json() as Budgets;
const failures: string[] = [];

function requireEqual(name: string, actual: string | number, expected: string | number) {
  if (actual !== expected) {
    failures.push(`${name}: expected ${expected}, measured ${actual}`);
  }
}

function check(limit: Limit) {
  const passes = limit.direction === "maximum"
    ? limit.actual <= limit.budget
    : limit.actual >= limit.budget;
  const relation = limit.direction === "maximum" ? "≤" : "≥";
  const summary = `${limit.name}: ${limit.actual}${limit.unit} (${relation} ${limit.budget}${limit.unit})`;
  console.log(`${passes ? "PASS" : "FAIL"} ${summary}`);
  if (!passes) failures.push(summary);
}

function maximum(name: string, actual: number, budget: number, unit: string) {
  check({ name, actual, budget, direction: "maximum", unit });
}

function minimum(name: string, actual: number, budget: number, unit: string) {
  check({ name, actual, budget, direction: "minimum", unit });
}

requireEqual("Android SDK", results.environment.sdk, budgets.environment.sdk);
requireEqual("ABI", results.environment.abi, budgets.environment.abi);
requireEqual(
  "refresh rate",
  results.environment.refreshRate,
  budgets.environment.refreshRate,
);
requireEqual("starting thermal status", results.environment.thermalStatusStart, 0);
requireEqual("ending thermal status", results.environment.thermalStatusEnd, 0);
const expectedResolution =
  `${budgets.environment.width}x${budgets.environment.height}`;
if (!results.environment.resolution.includes(expectedResolution)) {
  failures.push(
    `resolution: expected ${budgets.environment.width}x${budgets.environment.height}, measured ${results.environment.resolution}`,
  );
}

for (const scenario of ["Counter", "Scroll"] as const) {
  const result = results.results.find(
    (candidate) => candidate.stack === "Ink" && candidate.scenario === scenario,
  );
  if (!result) {
    failures.push(`missing Ink ${scenario} result`);
    continue;
  }
  const budget = scenario === "Counter" ? budgets.counter : budgets.scroll;
  const prefix = `Ink ${scenario}`;
  maximum(`${prefix} APK`, result.apk.bytes, budget.apkBytes, " B");
  maximum(
    `${prefix} start median`,
    result.startup.medianMs,
    budget.startupMedianMs,
    " ms",
  );
  maximum(
    `${prefix} start p95`,
    result.startup.p95Ms,
    budget.startupP95Ms,
    " ms",
  );
  maximum(
    `${prefix} idle PSS`,
    result.idleMemory.medianPssKb,
    budget.idlePssKb,
    " KiB",
  );
  maximum(
    `${prefix} idle RSS`,
    result.idleMemory.medianRssKb,
    budget.idleRssKb,
    " KiB",
  );
  maximum(
    `${prefix} threads`,
    result.idleMemory.medianThreads,
    budget.threads,
    "",
  );
  maximum(
    `${prefix} workload CPU`,
    result.workload.medianCpuMs,
    budget.workloadCpuMs,
    " ms",
  );
  if (budget.averageFps !== undefined) {
    minimum(
      `${prefix} cadence`,
      result.workload.medianAverageFps,
      budget.averageFps,
      " fps",
    );
  }
  if (budget.presentP95Ms !== undefined) {
    maximum(
      `${prefix} frame interval p95`,
      result.workload.medianPresentP95Ms,
      budget.presentP95Ms,
      " ms",
    );
  }
  if (scenario === "Scroll" && budget.continuousPresentP95Ms !== undefined) {
    const continuous = result.continuousScroll;
    if (!continuous) {
      failures.push("missing Ink continuous-scroll result");
      continue;
    }
    maximum(
      `${prefix} continuous p95`,
      continuous.medianPresentP95Ms,
      budget.continuousPresentP95Ms,
      " ms",
    );
    maximum(
      `${prefix} continuous p99`,
      continuous.medianPresentP99Ms,
      budget.continuousPresentP99Ms ?? budget.continuousPresentP95Ms,
      " ms",
    );
    maximum(
      `${prefix} continuous acquire p95`,
      continuous.medianAcquireP95Ms,
      budget.continuousAcquireP95Ms ?? budget.continuousPresentP95Ms,
      " ms",
    );
    maximum(
      `${prefix} continuous intervals >17ms`,
      continuous.medianLongPresentIntervals,
      budget.continuousLongPresentIntervals ?? 0,
      "",
    );
    maximum(
      `${prefix} continuous dropped frames`,
      continuous.medianDroppedFrames,
      budget.continuousDroppedFrames ?? 0,
      "",
    );
    maximum(
      `${prefix} continuous janky frames`,
      continuous.medianJankyFrames,
      budget.continuousJankyFrames ?? 0,
      "",
    );
    maximum(
      `${prefix} continuous late acquisitions`,
      continuous.medianLateAcquireFrames,
      budget.continuousLateAcquireFrames ?? 0,
      "",
    );
  }
}

if (buildPath) {
  const rows = (await Bun.file(buildPath).text())
    .trim()
    .split("\n")
    .slice(1)
    .map((line) => {
      const [round, stack, scenario, kind, seconds] = line.split(",");
      return { round, stack, scenario, kind, seconds: Number(seconds) };
    })
    .filter((row) => row.stack === "Ink");
  const median = (scenario: string, kind: string) => {
    const values = rows
      .filter((row) => row.scenario === scenario && row.kind === kind)
      .map((row) => row.seconds)
      .sort((a, b) => a - b);
    return values[Math.floor(values.length / 2)];
  };
  const buildLimits = [
    ["counter clean build", median("Counter", "clean"), budgets.build.counterCleanSeconds],
    ["scroll clean build", median("Scroll", "clean"), budgets.build.scrollCleanSeconds],
    ["scroll no-op build", median("Scroll", "noop"), budgets.build.scrollNoopSeconds],
  ] as const;
  for (const [name, actual, budget] of buildLimits) {
    if (actual === undefined) {
      failures.push(`missing Ink ${name} result`);
    } else {
      maximum(`Ink ${name}`, actual, budget, " s");
    }
  }
}

if (failures.length > 0) {
  console.error(`Ink performance budgets failed:\n${failures.join("\n")}`);
  process.exit(1);
}

console.log("Ink performance budgets passed");
