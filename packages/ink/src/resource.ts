import type { Snapshot, SnapshotSource } from "./snapshot";

export type ResourceSnapshot<T> =
  | Exclude<Snapshot<T>, { status: "ready" }>
  | { status: "ready"; data: T; refreshing: boolean; refreshError: Error | null };

export interface ResourceSource<T> extends SnapshotSource<T> {
  getSnapshot(): ResourceSnapshot<T>;
  refresh(): Promise<void>;
}

type Key = readonly (string | number | boolean | null)[];
const CACHE_TIME = 300_000;
const ERROR_BACKOFF = 60_000;

export function resource<Args extends unknown[], T>(options: {
  key: (...args: Args) => Key;
  load: (...args: Args) => Promise<T>;
  staleTime?: number;
  refreshInterval?: number;
}): (...args: Args) => ResourceSource<T> {
  const staleTime = options.staleTime ?? 0;
  const interval = options.refreshInterval;
  if (!Number.isFinite(staleTime) || staleTime < 0 ||
      (interval !== undefined && (!Number.isFinite(interval) || interval <= 0))) {
    throw new Error("Resource timings must be finite; staleTime must be non-negative and refreshInterval must be positive.");
  }
  const entries = new Map<string, ReturnType<typeof createEntry>>();

  function entryFor(key: string, args: Args) {
    let entry = entries.get(key);
    if (!entry) {
      entry = createEntry(key, args);
      entries.set(key, entry);
    }
    return entry;
  }

  function createEntry(key: string, args: Args) {
    let state: ResourceSnapshot<T> = { status: "loading" };
    let pending: Promise<void> | null = null;
    let updatedAt = 0;
    let retryAt = 0;
    let polling: ReturnType<typeof setTimeout> | undefined;
    let expiry: ReturnType<typeof setTimeout> | undefined;
    const listeners = new Set<() => void>();

    function publish(next: ResourceSnapshot<T>) {
      state = next;
      for (const listener of listeners) listener();
    }

    function expire() {
      if (expiry !== undefined) clearTimeout(expiry);
      expiry = setTimeout(() => {
        expiry = undefined;
        if (!listeners.size && !pending) {
          state = { status: "loading" };
          entries.delete(key);
        }
      }, CACHE_TIME);
    }

    function schedule() {
      if (polling !== undefined) clearTimeout(polling);
      polling = undefined;
      if (listeners.size && interval !== undefined && state.status === "ready" && !state.refreshError) {
        polling = setTimeout(() => { void refresh(true); }, Math.max(0, updatedAt + interval - Date.now()));
      }
    }

    function refresh(force = false): Promise<void> {
      if (pending) return pending;
      if (!force && (Date.now() < retryAt ||
          (state.status === "ready" && Date.now() < updatedAt + staleTime))) return Promise.resolve();
      if (polling !== undefined) clearTimeout(polling);
      polling = undefined;
      // Establish the pending request before notifying subscribers, which may call refresh.
      pending = Promise.resolve().then(() => options.load(...args)).then(data => {
        updatedAt = Date.now();
        retryAt = 0;
        publish({ status: "ready", data, refreshing: false, refreshError: null });
      }).catch(cause => {
        const error = cause instanceof Error ? cause : new Error(String(cause));
        retryAt = Date.now() + ERROR_BACKOFF;
        publish(state.status === "ready"
          ? { ...state, refreshing: false, refreshError: error }
          : { status: "error", error });
      }).finally(() => {
        pending = null;
        if (listeners.size) schedule();
        else expire();
      });
      publish(state.status === "ready"
        ? { ...state, refreshing: true, refreshError: null }
        : { status: "loading" });
      return pending;
    }

    function subscribe(listener: () => void) {
      if (expiry !== undefined) clearTimeout(expiry);
      expiry = undefined;
      listeners.add(listener);
      void refresh();
      if (!pending) schedule();
      return () => {
        listeners.delete(listener);
        if (!listeners.size) {
          if (polling !== undefined) clearTimeout(polling);
          polling = undefined;
          expire();
        }
      };
    }

    // Old handles still select the canonical entry after inactive data has expired.
    const source: ResourceSource<T> = {
      getSnapshot: () => entryFor(key, args).snapshot(),
      subscribe: listener => entryFor(key, args).subscribe(listener),
      refresh: () => entryFor(key, args).refresh(true),
    };
    expire();
    return { source, snapshot: () => state, subscribe, refresh };
  }

  return (...args) => {
    const parts = options.key(...args);
    if (parts.some(part => part !== null && typeof part !== "string" && typeof part !== "boolean" &&
        (typeof part !== "number" || !Number.isFinite(part)))) {
      throw new Error("Resource keys must contain only strings, finite numbers, booleans or null.");
    }
    return entryFor(JSON.stringify(parts), args).source;
  };
}
