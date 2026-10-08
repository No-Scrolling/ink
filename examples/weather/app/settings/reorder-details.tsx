import { ErrorState, ReorderList, Screen } from "ink";
import { PreferencesGate } from "../../components/PreferencesGate";
import { useSettingsUpdate } from "../../hooks/useSettingsUpdate";
import { DETAIL_LABELS, type WeatherPreferences } from "../../lib/preferences";

export default function ReorderDetailsScreen() {
  return (
    <PreferencesGate title="Reorder Details">
      {(prefs) => <ReorderDetails prefs={prefs} />}
    </PreferencesGate>
  );
}

function ReorderDetails({ prefs }: { prefs: WeatherPreferences }) {
  const save = useSettingsUpdate();
  if (save.status === "error")
    return (
      <Screen title="Reorder Details">
        <ErrorState message="Could not save the order." onRetry={save.retry} />
      </Screen>
    );
  return (
    <Screen title="Reorder Details">
      <ReorderList
        items={prefs.selectedDetails}
        keyExtractor={(detail) => detail}
        getLabel={(detail) => DETAIL_LABELS[detail]}
        onChange={(selectedDetails) => save.run({ selectedDetails })}
      />
    </Screen>
  );
}
