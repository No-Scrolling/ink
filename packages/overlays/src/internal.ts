import type { ReactNode } from "react";
import { onNativeMessage } from "ink/native";

export const overlayComponents = new Map<string, (props: object) => ReactNode>();
let content: { id: string; props: object } | null = null;
const listeners = new Set<() => void>();
onNativeMessage("overlay-content", message => {
  if (typeof message.id !== "string" || typeof message.props !== "object" || message.props === null || Array.isArray(message.props)) {
    throw new TypeError("Invalid overlay content");
  }
  content = { id: message.id, props: message.props };
  for (const listener of listeners) listener();
});

export const overlayContent = {
  getSnapshot: () => content,
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => { listeners.delete(listener); };
  },
};
