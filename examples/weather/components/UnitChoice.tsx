import { back, ErrorState, Screen, SettingsChoices } from "ink";
import { useSettingsUpdate } from "../hooks/useSettingsUpdate";
import type { WeatherPreferences } from "../lib/preferences";

export function UnitChoice<Value extends string>({
  title,
  value,
  options,
  labels,
  onSelect,
}: {
  title: string;
  value: Value;
  options: readonly Value[];
  labels?: Partial<Record<Value, string>>;
  onSelect: (value: Value) => Partial<WeatherPreferences>;
}) {
  const save = useSettingsUpdate({ onSuccess: back });
  if (save.status === "pending") return <Screen title={title} />;
  if (save.status === "error")
    return (
      <Screen title={title}>
        <ErrorState message="Could not save this setting." onRetry={save.retry} />
      </Screen>
    );
  return (
    <Screen title={title}>
      <SettingsChoices
        options={options.map((option) => ({ value: option, label: labels?.[option] ?? option }))}
        value={value}
        onChange={(next) => save.run(onSelect(next))}
      />
    </Screen>
  );
}
