import { useSyncExternalStore } from "react";

declare const __inkPost: (message: string) => void;

export type ColourScheme = "dark" | "light";
let colourScheme: ColourScheme = "dark";
const listeners = new Set<() => void>();

export function setColourScheme(value: ColourScheme): void {
  if (value !== "dark" && value !== "light") throw new TypeError("Invalid colour scheme");
  if (value === colourScheme) return;
  __inkPost(JSON.stringify({ type: "appearance", colourScheme: value }));
  colourScheme = value;
  for (const listener of listeners) listener();
}

export function useColourScheme(): ColourScheme {
  return useSyncExternalStore(subscribe, () => colourScheme);
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
