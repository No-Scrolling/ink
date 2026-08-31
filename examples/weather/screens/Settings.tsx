import { Field, Screen, Toggle, persistedState } from "ink";

export default function Settings() {
  const showIcons = persistedState("settings.show-icons", true);

  return (
    <Screen title="Settings (v0.1.0)">
      <Toggle
        label="Show Weather Icons"
        value={showIcons.value}
        onChange={() => showIcons.set(!showIcons.value)}
      />
      <Field label="Main Page Location">London</Field>
      <Field label="Temperature">Celsius</Field>
    </Screen>
  );
}
