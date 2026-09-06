import type { Snapshot, SnapshotSource } from "ink";
export { openDatabase, type SqlRow, type SqlValue } from "./database";
import { callNative, onNativeMessage } from "ink/native";

export interface Store<T> extends SnapshotSource<T> {
  get(): Promise<T>;
  set(value: T): Promise<void>;
  update(transform: (value: T) => T): Promise<void>;
  reset(): Promise<void>;
}

type Options<T> = {
  key: string;
  version: number;
  initial: T;
  decode(value: unknown): T;
  migrate?: (value: unknown, version: number) => unknown;
};
type Entry = { revision: number; version: number; value: string };
const observers = new Map<string, Set<() => void>>();
onNativeMessage("store-changed", message => {
  if (typeof message.key !== "string") throw new Error("Invalid store change event");
  for (const observer of observers.get(message.key) ?? []) observer();
});

async function read(key: string): Promise<Entry | null> {
  const result: unknown = JSON.parse(await callNative("store", "get", { key }));
  if (result === null) return null;
  if (typeof result !== "object" || !("revision" in result) || !("version" in result)
    || !("value" in result) || typeof result.revision !== "number"
    || !Number.isSafeInteger(result.revision) || result.revision < 1
    || typeof result.version !== "number" || !Number.isSafeInteger(result.version)
    || typeof result.value !== "string") throw new Error("Invalid stored entry");
  return { revision: result.revision, version: result.version, value: result.value };
}

async function write(key: string, revision: number, version: number, value: unknown): Promise<boolean> {
  const encoded = JSON.stringify(value);
  if (encoded === undefined) throw new Error("Store values must be JSON data");
  const result: unknown = JSON.parse(await callNative("store", "write", { key, revision, version, value: encoded }));
  if (typeof result !== "object" || result === null || !("committed" in result)
    || typeof result.committed !== "boolean") throw new Error("Invalid store write result");
  return result.committed;
}

export function createStore<T>(options: Options<T>): Store<T> {
  const { key, version, decode } = options;
  if (!key || key.length > 256 || !Number.isSafeInteger(version) || version < 1) {
    throw new Error("Invalid store key or version");
  }
  let snapshot: Snapshot<T> = { status: "loading" };
  const listeners = new Set<() => void>();
  let loading: Promise<T> | undefined;
  let generation = 0;

  function publish(next: Snapshot<T>) {
    snapshot = next;
    for (const listener of listeners) listener();
  }

  function decodeEntry(entry: Entry | null): T {
    if (!entry) return decode(options.initial);
    if (entry.version > version) throw new Error(`Store ${key} uses a newer version`);
    const value: unknown = JSON.parse(entry.value);
    if (entry.version === version) return decode(value);
    if (!options.migrate) throw new Error(`Store ${key} requires a migration`);
    return decode(options.migrate(value, entry.version));
  }

  async function load(): Promise<T> {
    for (let attempt = 0; attempt < 16; attempt++) {
      const entry = await read(key);
      const value = decodeEntry(entry);
      if (!entry || entry.version === version) return value;
      if (await write(key, entry.revision, version, value)) return value;
    }
    throw new Error(`Store ${key} changed too often; try again`);
  }

  function get(): Promise<T> {
    if (loading) return loading;
    const current = generation;
    const promise = load().then(data => {
      if (current === generation) publish({ status: "ready", data });
      return data;
    }, error => {
      if (current === generation) publish({ status: "error", error: error instanceof Error ? error : new Error(String(error)) });
      throw error;
    }).finally(() => { if (loading === promise) loading = undefined; });
    loading = promise;
    return loading;
  }

  function refresh() {
    generation++;
    loading = undefined;
    void get().catch(() => {});
  }

  async function mutate(transform: (entry: Entry | null) => T) {
    for (let attempt = 0; attempt < 16; attempt++) {
      const entry = await read(key);
      if (entry && entry.version > version) throw new Error(`Store ${key} uses a newer version`);
      const value = decode(transform(entry));
      if (await write(key, entry?.revision ?? 0, version, value)) {
        generation++;
        loading = undefined;
        publish({ status: "ready", data: value });
        for (const observer of observers.get(key) ?? []) observer();
        return;
      }
    }
    throw new Error(`Store ${key} changed too often; try again`);
  }

  return {
    get,
    set: value => mutate(() => value),
    update: transform => mutate(entry => transform(decodeEntry(entry))),
    reset: () => mutate(() => options.initial),
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      if (listeners.size === 1) {
        let group = observers.get(key);
        if (!group) observers.set(key, group = new Set());
        group.add(refresh);
        refresh();
      }
      return () => {
        listeners.delete(listener);
        if (!listeners.size) {
          const group = observers.get(key);
          group?.delete(refresh);
          if (!group?.size) observers.delete(key);
        }
      };
    },
  };
}
