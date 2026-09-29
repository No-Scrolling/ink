import { callNative } from "ink/native";
import type { ViewCollection } from "ink/view";

export type SqlValue = string | number | null;
export type SqlRow = Readonly<Record<string, SqlValue>>;

export async function openDatabase(asset: string) {
  const id = await callNative("sqlite", "open", { asset });
  let closed = false;
  let closing: Promise<void> | undefined;
  return {
    async queryInto<T extends SqlRow>(collection: ViewCollection<T>, sql: string, parameters: readonly SqlValue[] = [], options: { signal?: AbortSignal } = {}): Promise<number> {
      if (closed || closing) throw new Error("Database is closed");
      if (parameters.some(value => value !== null && typeof value !== "string" && (typeof value !== "number" || !Number.isFinite(value)))) throw new TypeError("SQL parameters must be strings, finite numbers or null");
      return Number(await callNative("sqlite", "query-collection", { id, sql, parameters, target: collection.reserve() }, options));
    },
    async query(sql: string, parameters: readonly SqlValue[] = [], options: { signal?: AbortSignal } = {}): Promise<readonly SqlRow[]> {
      if (closed || closing) throw new Error("Database is closed");
      if (parameters.some(value => value !== null && typeof value !== "string" && (typeof value !== "number" || !Number.isFinite(value)))) throw new TypeError("SQL parameters must be strings, finite numbers or null");
      const result: unknown = JSON.parse(await callNative("sqlite", "query", { id, sql, parameters }, options));
      if (!Array.isArray(result) || result.some(row => typeof row !== "object" || row === null || Array.isArray(row)
        || Object.values(row).some(value => value !== null && typeof value !== "string" && typeof value !== "number"))) {
        throw new Error("Invalid database result");
      }
      return result;
    },
    async close(): Promise<void> {
      if (closed) return;
      if (closing) return closing;
      closing = callNative("sqlite", "close", { id }).then(() => { closed = true; });
      try { await closing; } finally { closing = undefined; }
    },
  };
}
