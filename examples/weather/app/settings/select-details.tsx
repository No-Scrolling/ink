import { Button, ErrorState, List, Screen } from "ink";
import { PreferencesGate } from "../../components/PreferencesGate";
import { useSettingsUpdate } from "../../hooks/useSettingsUpdate";
import { DETAIL_LABELS, WEATHER_DETAILS, type WeatherDetail, type WeatherPreferences } from "../../lib/preferences";

export default function SelectDetailsScreen() {
  return <PreferencesGate title="Details">{prefs => <SelectDetails prefs={prefs} />}</PreferencesGate>;
}

function SelectDetails({ prefs }: { prefs: WeatherPreferences }) {
  const save = useSettingsUpdate();
  const toggle = (detail: WeatherDetail) => {
    const selected = prefs.selectedDetails.includes(detail);
    if (selected && prefs.selectedDetails.length === 1) return;
    const selectedDetails = selected
      ? prefs.selectedDetails.filter(value => value !== detail)
      : [...prefs.selectedDetails, detail];
    save.run({ selectedDetails });
  };
  if (save.status === "error") return <Screen title="Details"><ErrorState message="Could not save the details." onRetry={save.retry} /></Screen>;
  return (
    <Screen title="Details">
      <List
        items={WEATHER_DETAILS}
        keyExtractor={detail => detail}
        renderItem={detail => (
          <Button
            selected={prefs.selectedDetails.includes(detail)}
            onPress={() => toggle(detail)}
          >
            {DETAIL_LABELS[detail]}
          </Button>
        )}
      />
    </Screen>
  );
}
