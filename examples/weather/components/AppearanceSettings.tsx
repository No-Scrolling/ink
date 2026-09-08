import { setColourScheme, useSnapshot } from "ink";
import { useEffect } from "react";
import { preferences } from "../lib/preferences";

export function AppearanceSettings() {
  const saved = useSnapshot(preferences);
  useEffect(() => {
    if (saved.status === "ready") setColourScheme(saved.data.invertColours ? "light" : "dark");
  }, [saved]);
  return null;
}
