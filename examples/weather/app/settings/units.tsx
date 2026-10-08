import { PreferencesGate } from "../../components/PreferencesGate";
import { Field, Screen } from "ink";

export default function UnitsScreen() {
  return (
    <PreferencesGate title="Units">
      {(prefs) => (
        <Screen title="Units">
          <Field label="Temperature" href="/settings/temperature">
            {prefs.temperatureUnit}
          </Field>
          <Field label="Wind Speed" href="/settings/wind-speed">
            {prefs.windSpeedUnit}
          </Field>
          <Field label="Precip." href="/settings/precipitation">
            {prefs.precipitationUnit === "Millimeter" ? "Millimetres" : "Inches"}
          </Field>
          <Field label="Time Format" href="/settings/time-format">
            {prefs.timeFormat === "24h" ? "24 Hour" : "12 Hour"}
          </Field>
        </Screen>
      )}
    </PreferencesGate>
  );
}
