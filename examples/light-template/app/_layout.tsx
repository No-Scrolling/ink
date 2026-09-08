import { Slot } from "ink";
import { AppearanceSettings } from "../data/appearance";

export default function Layout() {
  return <><AppearanceSettings /><Slot /></>;
}
