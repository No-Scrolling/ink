import { createElement, type ComponentType } from "ink/internal/react";
import type { Snapshot, SnapshotSource } from "ink";
import { callNative, onNativeMessage } from "ink/native";
import { overlayComponents } from "./internal";

export type OverlayPosition = "top" | "center" | "bottom";
export type OverlayOptions = { width?: number; height: number; position?: OverlayPosition; offset?: number };
export type OverlayState = Readonly<{ visible: boolean; id: string | null }>;

let snapshot: Snapshot<OverlayState> = { status: "loading" };
const listeners = new Set<() => void>();
function publish(data: OverlayState) {
  snapshot = { status: "ready", data: Object.freeze(data) };
  for (const listener of listeners) listener();
}
function decode(value: unknown): OverlayState {
  if (!value || typeof value !== "object" || !("visible" in value) || typeof value.visible !== "boolean"
    || !("id" in value) || (value.id !== null && typeof value.id !== "string")) throw new Error("Invalid overlay state");
  return { visible: value.visible, id: value.id };
}
onNativeMessage("overlays-changed", message => { publish(decode(message.data)); });
async function read(operation: string): Promise<OverlayState> {
  try {
    const data = decode(JSON.parse(await callNative("overlays", operation, {})));
    publish(data);
    return data;
  } catch (error) {
    snapshot = { status: "error", error: error instanceof Error ? error : new Error(String(error)) };
    for (const listener of listeners) listener();
    throw error;
  }
}

export function defineOverlay<Props extends object>(definition: { id: string; component: ComponentType<Props> }) {
  if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,127}$/.test(definition.id)) throw new TypeError("Invalid overlay ID");
  if (overlayComponents.has(definition.id)) throw new Error(`Duplicate overlay: ${definition.id}`);
  overlayComponents.set(definition.id, props => createElement(definition.component, props as Props));
  return {
    async show(props: Props, options: OverlayOptions): Promise<void> {
      await callNative("overlays", "show", { id: definition.id, props, options });
    },
    async update(props: Props, options?: OverlayOptions): Promise<void> {
      await callNative("overlays", "update", { id: definition.id, props, options });
    },
    async hide(): Promise<void> {
      await callNative("overlays", "hide", { id: definition.id });
    },
  };
}

export const overlays: SnapshotSource<OverlayState> & {
  isAllowed(): Promise<boolean>;
  requestPermission(): Promise<void>;
  getState(): Promise<OverlayState>;
} = {
  isAllowed: async () => (await callNative("overlays", "permission", {})) === "true",
  async requestPermission(): Promise<void> {
    await callNative("overlays", "request-permission", {}, { timeoutMs: 900_000 });
  },
  getState: () => read("state"),
  getSnapshot: () => snapshot,
  subscribe(listener) {
    listeners.add(listener);
    if (listeners.size === 1) void read("watch").catch(() => {});
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0) void callNative("overlays", "unwatch", {}).catch(() => {});
    };
  },
};
