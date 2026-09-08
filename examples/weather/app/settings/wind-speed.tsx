import { PreferencesGate } from "../../components/PreferencesGate";
import { UnitChoice } from "../../components/UnitChoice";

export default function WindSpeedUnitScreen() {
  return <PreferencesGate title="Wind Speed">{prefs => <UnitChoice title="Wind Speed" value={prefs.windSpeedUnit} options={["km/h", "m/s", "mph", "Knots"]} onSelect={value => ({ windSpeedUnit: value })} />}</PreferencesGate>;
}
