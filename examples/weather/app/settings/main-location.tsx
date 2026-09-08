import { back, Button, ErrorState, Screen, Text } from "ink";
import { PreferencesGate } from "../../components/PreferencesGate";
import { useSettingsUpdate } from "../../hooks/useSettingsUpdate";
import { formatLocationName, type WeatherPreferences } from "../../lib/preferences";

export default function MainLocationScreen() {
  return <PreferencesGate title="Main Page Location">{prefs => <MainLocation prefs={prefs} />}</PreferencesGate>;
}

function MainLocation({ prefs }: { prefs: WeatherPreferences }) {
  const save = useSettingsUpdate({ onSuccess: back });
  const selected = prefs.mainLocation ? `${prefs.mainLocation.id}:${prefs.mainLocation.latitude}:${prefs.mainLocation.longitude}` : "current";
  if (save.status === "pending") return <Screen title="Main Page Location" />;
  if (save.status === "error") return <Screen title="Main Page Location"><ErrorState message="Could not save this setting." onRetry={save.retry} /></Screen>;
  return (
    <Screen title="Main Page Location">
      <Button selected={selected === "current"} onPress={() => save.run({ mainLocation: null })}>Current Location</Button>
      {prefs.savedLocations.map(saved => {
        const key = `${saved.id}:${saved.latitude}:${saved.longitude}`;
        return <Button key={key} selected={selected === key} onPress={() => save.run({ mainLocation: saved })}>{formatLocationName(saved)}</Button>;
      })}
      {prefs.savedLocations.length === 0 && <Text size={18}>Add a location in Locations to use it on the main page.</Text>}
    </Screen>
  );
}
