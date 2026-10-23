export const coreWorkloads = [
  ...["small", "bulk", "resize"].map(command => `react-${command}`),
  ...[100, 1000, 10000].flatMap(count => [`list-edit-${count}`, `list-scroll-${count}`]),
  "scrollbar-drag-10000",
  "rows-edit-100", "rows-edit-1000", "rows-prepend-1000", "rows-reorder-1000",
  "rows-native-scroll-1000", "rows-react-scroll-1000", "rows-pure-scroll-1000",
  "rows-decoded-image-arrival", "startup-runtime-to-library",
  "navigation-open", "navigation-back", "navigation-tab",
  "conversation-controlled-input", "conversation-append", "conversation-prepend",
  "conversation-input-background", "conversation-input-under-load",
  "playback-native-clock", "playback-state-change", "conversation-input-cpu",
];

const rendererCoverage = [
  "cold-multilingual-glyphs", "multilingual-atlas-turnover",
  "sustained-full-frame-uploads", "sustained-variable-frame-uploads",
  "artwork-arrival-scroll-turnover",
];

export const suites = {
  core: {
    binary: "ink-performance",
    assets: ["react", "list", "rows", "navigation", "conversation", "playback"],
    workloads: coreWorkloads,
  },
  source: {
    binary: "ink-source-performance",
    assets: ["source"],
    workloads: [1000, 5000].flatMap(count => ["edit", "replace"].map(command => `source-${command}-${count}`)),
  },
  automatic: {
    binary: "ink-automatic-performance",
    assets: ["automatic"],
    workloads: [
      ...[1000, 5000].flatMap(count => ["edit", "replace"].map(command => `ordinary-${command}-${count}`)),
      "typing-during-refresh-5000",
    ],
  },
  journeys: {
    binary: "ink-journey-performance",
    assets: ["journeys"],
    workloads: [
      ...[1000, 5000, 10000].flatMap(count => [
        ...["insert", "delete", "reorder", "resize"].map(command => `journey-list-${command}-${count}`),
        `journey-local-search-unprepared-${count}`, `journey-local-search-cache-ready-${count}`,
      ]),
      "journey-sqlite-search-continuous-cancelled", "journey-route-loading-error-retry-back",
      "journey-persisted-startup-fresh-process",
    ],
  },
  renderer: {
    binary: "ink-render-performance",
    assets: ["rows"],
    workloads: [
      "gif-frame-thumbnail", "gif-frame-full-width", "first-frame-rows", "first-frame-dense-text",
      "atlas-recovery-1-mib", "atlas-recovery-4-mib", "atlas-recovery-16-mib",
      "artwork-revisit", "distant-text-edits", "artwork-changing-hot-set",
      "artwork-mixed-reconstruction", "artwork-large-covers", ...rendererCoverage,
    ],
  },
};

export type Suite = keyof typeof suites;
export type Selection = { suites: Suite[]; coverageOnly: boolean };
export type Sample = Record<string, unknown>;
export type Workload = {
  name: string;
  samples?: Sample[];
  raw_samples?: Sample[];
  jobs?: { list_ready_ns: number; inputs: Sample[] }[];
  contracts?: unknown;
};
export type Run = {
  workloads: Workload[];
  contracts?: Record<string, unknown>;
  idle?: { commits: number };
  [key: string]: unknown;
};

export function selectSuites(name: string): Selection {
  if (name === "all") return { suites: Object.keys(suites) as Suite[], coverageOnly: false };
  if (name === "coverage") return { suites: ["journeys", "renderer"], coverageOnly: true };
  if (name === "device") return { suites: [], coverageOnly: false };
  if (Object.hasOwn(suites, name)) return { suites: [name as Suite], coverageOnly: false };
  throw Error("Suite must be all, core, source, automatic, journeys, renderer, coverage or device");
}

export function workloadNames(suite: Suite, coverageOnly: boolean): string[] {
  return suite === "renderer" && coverageOnly ? rendererCoverage : suites[suite].workloads;
}

export function workloadSamples(workload: Workload): Sample[] {
  return workload.jobs ?? workload.raw_samples ?? workload.samples ?? [];
}

export function primaryMetric(suite: Suite, workload: Workload): string {
  return suite === "renderer" ? "cpu_ns" : workload.jobs ? "list_ready_ns" : "elapsed_ns";
}

function sampleCount(name: string, requested: number): number {
  if (name === "cold-multilingual-glyphs") return Math.min(requested, 16);
  if (name === "multilingual-atlas-turnover") return Math.max(requested, 12);
  if (name.startsWith("sustained-") || name === "artwork-arrival-scroll-turnover") return Math.max(requested, 120);
  if (name.startsWith("first-frame-")) return Math.min(requested, 30);
  if (name.startsWith("atlas-recovery-")) return Math.min(requested, 40);
  if (name === "artwork-revisit") return Math.min(requested, 12);
  if (["artwork-changing-hot-set", "artwork-mixed-reconstruction", "artwork-large-covers"].includes(name) || name.startsWith("typing-during-refresh-")) return Math.min(requested, 8);
  return requested;
}

export function validateRun(suite: Suite, run: Run, samples: number, coverageOnly: boolean): void {
  if (!Array.isArray(run.workloads) || JSON.stringify(run.workloads.map(work => work.name)) !== JSON.stringify(workloadNames(suite, coverageOnly))) throw Error(`Missing or unexpected ${suite} workloads`);
  if (suite === "core" && run.idle?.commits !== 0) throw Error("Core idle contract did not pass");
  if (suite === "automatic" && !["latest_requested_callback", "supersession", "unmount_remount", "bounded_native_window"].every(key => run.contracts?.[key] === true)) throw Error("Automatic list lifecycle contracts did not pass");
  for (const work of run.workloads) {
    const rows = workloadSamples(work), field = primaryMetric(suite, work);
    if (rows.length !== sampleCount(work.name, samples) || rows.some(row => typeof row[field] !== "number" || !Number.isFinite(row[field]) || row[field] <= 0)) throw Error(`Invalid samples: ${work.name}`);
    if ((suite === "journeys" || suite === "renderer" || suite === "source") && !work.contracts) throw Error(`Missing contracts: ${work.name}`);
    if (work.jobs?.some(job => job.inputs.length !== 16 || job.inputs.some(input => !["arrival_to_ack_ns", "dispatch_to_ack_ns"].every(key => typeof input[key] === "number" && Number.isFinite(input[key]) && input[key] > 0)))) throw Error(`Incomplete typing acknowledgements: ${work.name}`);
  }
}
