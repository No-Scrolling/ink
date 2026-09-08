import { Slot } from "ink";
import { AppearanceSettings } from "../components/AppearanceSettings";

export default function Layout() {
  return <><AppearanceSettings /><Slot /></>;
}
