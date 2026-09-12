import { createContext, useContext, useState, type ReactNode } from "react";
import { resource } from "ink";

const Settings = createContext({ alternate: false, toggle: () => {} });

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [alternate, setAlternate] = useState(false);
  return <Settings value={{ alternate, toggle: () => setAlternate(value => !value) }}>{children}</Settings>;
}

export const useSettings = () => useContext(Settings);

let attempt = 0;
export const catalogue = resource({
  key: () => [],
  staleTime: 3_600_000,
  load: async () => {
    const revision = ++attempt;
    await new Promise<void>(resolve => setTimeout(resolve, 150));
    if (revision % 2 === 0) throw new Error("Fixture refresh failure");
    return revision;
  },
});
