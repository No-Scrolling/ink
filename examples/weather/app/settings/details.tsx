import { Button, Field, Screen } from "ink";
import { PreferencesGate } from "../../components/PreferencesGate";
import type { WeatherPreferences } from "../../lib/preferences";

export default function DetailsSettingsScreen() {
  return (
    <PreferencesGate title="Weather Details">
      {(prefs) => <DetailsSettings prefs={prefs} />}
    </PreferencesGate>
  );
}

function DetailsSettings({ prefs }: { prefs: WeatherPreferences }) {
  return (
    <Screen title="Weather Details">
      <Field label="Weather Details" href="/settings/select-details">
        {prefs.selectedDetails.length} selected
      </Field>
      <Button href="/settings/reorder-details">Reorder Details</Button>
    </Screen>
  );
}
