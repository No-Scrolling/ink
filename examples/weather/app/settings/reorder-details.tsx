import { Button, ErrorState, Screen, Stack, Text } from "ink";
import { keyboardArrowDown, keyboardArrowUp } from "ink/icons";
import { PreferencesGate } from "../../components/PreferencesGate";
import { useSettingsUpdate } from "../../hooks/useSettingsUpdate";
import { DETAIL_LABELS, type WeatherPreferences } from "../../lib/preferences";

export default function ReorderDetailsScreen() {
  return <PreferencesGate title="Reorder Details">{prefs => <ReorderDetails prefs={prefs} />}</PreferencesGate>;
}

function ReorderDetails({ prefs }: { prefs: WeatherPreferences }) {
  const save = useSettingsUpdate();
  const move = (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (target < 0 || target >= prefs.selectedDetails.length) return;
    const selectedDetails = [...prefs.selectedDetails];
    [selectedDetails[index], selectedDetails[target]] = [selectedDetails[target], selectedDetails[index]];
    save.run({ selectedDetails });
  };
  if (save.status === "error") return <Screen title="Reorder Details"><ErrorState message="Could not save the order." onRetry={save.retry} /></Screen>;
  return (
    <Screen title="Reorder Details">
      {prefs.selectedDetails.map((detail, index) => (
        <Stack key={detail} axis="horizontal" align="center" justify="space-between">
          <Text maxLines={1}>{DETAIL_LABELS[detail]}</Text>
          <Stack axis="horizontal" align="center" gap={4}>
            <Button icon={keyboardArrowDown} disabled={index === prefs.selectedDetails.length - 1} onPress={() => move(index, 1)} />
            <Button icon={keyboardArrowUp} disabled={index === 0} onPress={() => move(index, -1)} />
          </Stack>
        </Stack>
      ))}
    </Screen>
  );
}
