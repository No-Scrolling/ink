import { useEffect } from "react";
import { setColourScheme, useSnapshot, type ColourScheme } from "ink";
import { createStore } from "@ink/store";

export const appearance = createStore<ColourScheme>({
  key: "appearance",
  version: 1,
  initial: "dark",
  decode(value) {
    if (value !== "dark" && value !== "light") throw new Error("Invalid saved appearance");
    return value;
  },
});

export function AppearanceSettings() {
  const saved = useSnapshot(appearance);
  useEffect(() => {
    if (saved.status === "ready") setColourScheme(saved.data);
  }, [saved]);
  return null;
}
