import type { Snapshot, SnapshotSource } from "ink";
import { callNative, onNativeMessage } from "ink/native";

export type ConnectionState = Readonly<{
  status: "unknown" | "offline" | "connected";
  transport?: "wifi" | "cellular" | "ethernet" | "vpn" | "other";
  metered?: boolean;
}>;
let snapshot: Snapshot<ConnectionState> = { status: "loading" };
const listeners = new Set<() => void>();
function publish(data: ConnectionState) {
  snapshot = { status: "ready", data: Object.freeze(data) };
  for (const listener of listeners) listener();
}
function decode(value: unknown): ConnectionState {
  if (!value || typeof value !== "object" || !("status" in value) || !["unknown", "offline", "connected"].includes(String(value.status))) throw new Error("Invalid connectivity snapshot");
  return value as ConnectionState;
}
onNativeMessage("connectivity-changed", message => { publish(decode(message.data)); });
async function read(operation: string): Promise<ConnectionState> {
  try {
    const data = decode(JSON.parse(await callNative("connectivity", operation, {})));
    publish(data);
    return data;
  } catch (error) {
    snapshot = { status: "error", error: error instanceof Error ? error : new Error(String(error)) };
    for (const listener of listeners) listener();
    throw error;
  }
}
export const connectivity: SnapshotSource<ConnectionState> & { get(): Promise<ConnectionState> } = {
  get: () => read("snapshot"),
  getSnapshot: () => snapshot,
  subscribe(listener) {
    listeners.add(listener);
    if (listeners.size === 1) void read("watch").catch(() => {});
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) void callNative("connectivity", "unwatch", {}).catch(() => {});
    };
  },
};
