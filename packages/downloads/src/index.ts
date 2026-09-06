import type { Snapshot, SnapshotSource } from "ink";
import { callNative } from "ink/native";
import { attachNativeController } from "ink/native/controller";
import type { FileRef } from "@ink/files";

export type DownloadState = {
  id: string;
  state: "queued" | "running" | "paused" | "completed" | "failed" | "cancelled";
  received: number;
  total: number | null;
  file: FileRef | null;
  error: string | null;
};
function decode(value: unknown): DownloadState {
  const byteCount = (number: unknown) => typeof number === "number" && Number.isSafeInteger(number) && number >= 0;
  if (typeof value !== "object" || value === null || !("id" in value) || typeof value.id !== "string"
    || !("state" in value) || !["queued", "running", "paused", "completed", "failed", "cancelled"].includes(String(value.state))
    || !("received" in value) || !byteCount(value.received)
    || !("total" in value) || (value.total !== null && !byteCount(value.total))
    || !("file" in value) || !("error" in value) || (value.error !== null && typeof value.error !== "string")) {
    throw new Error("Invalid download state");
  }
  if (value.file !== null) {
    const file = value.file;
    if (typeof file !== "object" || Array.isArray(file)
      || !["id", "src", "name", "mimeType"].every(key => typeof Reflect.get(file, key) === "string")
      || !byteCount(Reflect.get(file, "size"))) throw new Error("Invalid downloaded file");
    for (const key of ["width", "height", "duration"]) {
      const number: unknown = Reflect.get(file, key);
      if (number !== undefined && !byteCount(number)) throw new Error("Invalid downloaded media metadata");
    }
  }
  if (value.state === "completed" && value.file === null) throw new Error("Completed download has no file");
  return value as DownloadState;
}
function observe(id: string): SnapshotSource<DownloadState> {
  let snapshot: Snapshot<DownloadState> = { status: "loading" };
  const listeners = new Set<() => void>();
  let attachment: ReturnType<typeof attachNativeController> | undefined;
  const publish = (value: Snapshot<DownloadState>) => { snapshot = value; for (const listener of listeners) listener(); };
  return {
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      if (!attachment) {
        const current = attachNativeController("downloads", { id }, value => {
          if (attachment !== current) return;
          try { publish({ status: "ready", data: decode(value) }); }
          catch (error) { publish({ status: "error", error: error instanceof Error ? error : new Error(String(error)) }); }
        });
        attachment = current;
        void current.ready.catch(error => {
          if (attachment === current) publish({ status: "error", error });
        });
      }
      return () => {
        listeners.delete(listener);
        if (!listeners.size) {
          const current = attachment;
          attachment = undefined;
          snapshot = { status: "loading" };
          void current?.dispose().catch(error => console.error("Could not stop observing download", error));
        }
      };
    },
  };
}
const command = async (operation: string, id: string): Promise<void> => {
  await callNative("downloads", operation, { id });
};
export const downloads = {
  async enqueue(options: { key: string; url: string; name: string; network?: "connected" | "unmetered" }): Promise<DownloadState> {
    return decode(JSON.parse(await callNative("downloads", "enqueue", options)));
  },
  async get(id: string): Promise<DownloadState> {
    return decode(JSON.parse(await callNative("downloads", "get", { id })));
  },
  observe,
  pause: (id: string) => command("pause", id),
  resume: (id: string) => command("resume", id),
  cancel: (id: string) => command("cancel", id),
  remove: (id: string) => command("remove", id),
};
