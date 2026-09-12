import { Slot } from "ink";
import { SettingsProvider } from "../lib/state";

export default function Layout() {
  return <SettingsProvider><Slot /></SettingsProvider>;
}
