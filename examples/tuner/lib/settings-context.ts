import { createContext, useContext } from "react";
import type { Preferences } from "./preferences";

export const SettingsContext = createContext<Preferences | null>(null);

export function useSettings() {
  const settings = useContext(SettingsContext);
  if (!settings) throw new Error("Settings are unavailable outside the app layout");
  return settings;
}
