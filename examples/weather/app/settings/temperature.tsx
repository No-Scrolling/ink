import { PreferencesGate } from "../../components/PreferencesGate";
import { UnitChoice } from "../../components/UnitChoice";

export default function TemperatureUnitScreen() {
  return (
    <PreferencesGate title="Temperature">
      {(prefs) => (
        <UnitChoice
          title="Temperature"
          value={prefs.temperatureUnit}
          options={["Celsius", "Fahrenheit"]}
          onSelect={(value) => ({ temperatureUnit: value })}
        />
      )}
    </PreferencesGate>
  );
}
