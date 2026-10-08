import type { Snapshot, SnapshotSource } from "ink";
import { callNative } from "ink/native";

export type TaskResult = { status: "success" } | { status: "retry"; delayMs?: number } | { status: "failed"; reason: string };
export interface EnqueueOptions {
  key: string;
  intervalMinutes?: number;
  constraints?: { network?: "connected"; charging?: boolean };
}
const tasks = new Map<string, (input: unknown, signal: AbortSignal) => Promise<TaskResult>>();
export function defineTask<Input>(definition: {
  id: string;
  decode: (input: unknown) => Input;
  run: (context: { input: Input; signal: AbortSignal }) => TaskResult | Promise<TaskResult>;
}) {
  if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$/.test(definition.id)) throw new TypeError("Invalid background task ID");
  if (tasks.has(definition.id)) throw new Error(`Duplicate background task: ${definition.id}`);
  tasks.set(definition.id, async (input, signal) => definition.run({ input: definition.decode(input), signal }));
  return {
    id: definition.id,
    async enqueue(input: Input, options: EnqueueOptions): Promise<void> {
      const decoded = definition.decode(input);
      if (!options.key || options.key.length > 256) throw new TypeError("Background task key must contain 1–256 characters");
      if (options.intervalMinutes !== undefined && (!Number.isSafeInteger(options.intervalMinutes) || options.intervalMinutes < 15)) {
        throw new RangeError("Periodic background work requires at least fifteen minutes");
      }
      await callNative("background", "enqueue-task", { task: definition.id, input: decoded, ...options });
    },
  };
}
export async function cancel(key: string): Promise<void> {
  await callNative("background", "cancel-task", { key });
}
export async function runRegisteredTask(id: string, input: unknown, signal: AbortSignal): Promise<TaskResult> {
  const task = tasks.get(id);
  if (!task) return { status: "failed", reason: "unknown-task" };
  const result = await task(input, signal);
  if (result.status === "success") return result;
  if (result.status === "failed" && typeof result.reason === "string" && result.reason.length > 0 && result.reason.length <= 256) return result;
  if (result.status === "retry" && (result.delayMs === undefined || (Number.isSafeInteger(result.delayMs) && result.delayMs >= 0))) return result;
  throw new TypeError("Invalid background task result");
}

export interface JobState {
  key: string;
  task: string;
  status: "queued" | "running" | "retrying" | "succeeded" | "failed" | "cancelled";
  updatedAt: number;
  reason: string | null;
  scheduled: boolean;
  periodic: boolean;
}
interface JobSnapshot { revision: number; jobs: JobState[] }
function decodeJobs(source: string): JobSnapshot {
  const value: unknown = JSON.parse(source);
  if (typeof value !== "object" || value === null || !("revision" in value) || typeof value.revision !== "number"
    || !("jobs" in value) || !Array.isArray(value.jobs)) throw new TypeError("Invalid background job snapshot");
  const jobs = value.jobs.map((job: unknown): JobState => {
    if (typeof job !== "object" || job === null || !("key" in job) || typeof job.key !== "string"
      || !("task" in job) || typeof job.task !== "string" || !("status" in job)
      || (job.status !== "queued" && job.status !== "running" && job.status !== "retrying" && job.status !== "succeeded" && job.status !== "failed" && job.status !== "cancelled")
      || !("updatedAt" in job) || typeof job.updatedAt !== "number"
      || !("reason" in job) || (job.reason !== null && typeof job.reason !== "string")
      || !("scheduled" in job) || typeof job.scheduled !== "boolean"
      || !("periodic" in job) || typeof job.periodic !== "boolean") throw new TypeError("Invalid background job state");
    return { key: job.key, task: job.task, status: job.status, updatedAt: job.updatedAt, reason: job.reason, scheduled: job.scheduled, periodic: job.periodic };
  });
  return { revision: value.revision, jobs };
}
async function getJobs(options: { signal?: AbortSignal } = {}): Promise<readonly JobState[]> {
  return decodeJobs(await callNative("background", "task-state", {}, options)).jobs;
}
export async function* watchJobs(options: { signal?: AbortSignal } = {}): AsyncGenerator<readonly JobState[]> {
  let revision = -1;
  for (;;) {
    const snapshot = decodeJobs(await callNative("background", "observe-task-state", { revision }, options));
    if (snapshot.revision !== revision) yield snapshot.jobs;
    revision = snapshot.revision;
  }
}

let jobsSnapshot: Snapshot<readonly JobState[]> = { status: "loading" };
const jobListeners = new Set<() => void>();
let jobObservation: AbortController | undefined;
function publishJobs(snapshot: Snapshot<readonly JobState[]>) {
  jobsSnapshot = snapshot;
  for (const listener of jobListeners) listener();
}
export const jobs: SnapshotSource<readonly JobState[]> & { get: typeof getJobs } = {
  get: getJobs,
  getSnapshot: () => jobsSnapshot,
  subscribe(listener) {
    jobListeners.add(listener);
    if (!jobObservation) {
      const controller = new AbortController();
      jobObservation = controller;
      void (async () => {
        try {
          for await (const data of watchJobs({ signal: controller.signal })) {
            if (jobObservation === controller) publishJobs({ status: "ready", data });
          }
        } catch (error) {
          if (jobObservation === controller) publishJobs({ status: "error", error: error instanceof Error ? error : new Error(String(error)) });
        }
      })();
    }
    return () => {
      jobListeners.delete(listener);
      if (!jobListeners.size) {
        const controller = jobObservation;
        jobObservation = undefined;
        jobsSnapshot = { status: "loading" };
        controller?.abort();
      }
    };
  },
};
