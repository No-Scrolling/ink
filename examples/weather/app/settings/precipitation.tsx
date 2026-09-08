import { PreferencesGate } from "../../components/PreferencesGate";
import { UnitChoice } from "../../components/UnitChoice";

export default function PrecipitationUnitScreen() {
  return <PreferencesGate title="Precip.">{prefs => <UnitChoice title="Precip." value={prefs.precipitationUnit} options={["Millimeter", "Inch"]} labels={{ Millimeter: "Millimetres", Inch: "Inches" }} onSelect={value => ({ precipitationUnit: value })} />}</PreferencesGate>;
}
