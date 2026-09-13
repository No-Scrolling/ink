import { Button, ErrorState, Field, Screen, Toggle } from "ink";
import { PreferencesGate } from "../../components/PreferencesGate";
import { useSettingsUpdate } from "../../hooks/useSettingsUpdate";
import { formatLocationName, type WeatherPreferences } from "../../lib/preferences";

export default function SettingsScreen() {
  return <PreferencesGate title="Settings">{prefs => <Settings prefs={prefs} />}</PreferencesGate>;
}

function Settings({ prefs }: { prefs: WeatherPreferences }) {
  const save = useSettingsUpdate();
  const mainLocationLabel = prefs.mainLocation ? formatLocationName(prefs.mainLocation) : "Current Location";
  if (save.status === "error") return <Screen title="Settings"><ErrorState message="Could not save settings." onRetry={save.retry} /></Screen>;
  return (
    <Screen title="Settings">
      <Toggle label="Invert Colours" value={prefs.invertColours} onChange={value => save.run({ invertColours: value })} />
      <Field label="Main Page Location" href="/settings/main-location">{mainLocationLabel}</Field>
      <Button href="/settings/details">Weather Details</Button>
      <Button href="/settings/units">Units</Button>
    </Screen>
  );
}
