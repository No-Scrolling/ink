import { Screen, SelectorButton, Toggle, persistedState } from "ink";

export default function Settings() {
  const showIcons = persistedState("settings.show-icons", true);

  return (
    <Screen title="Settings (v0.1.0)">
      <Toggle
        label="Show Weather Icons"
        value={showIcons.value}
        onChange={() => showIcons.set(!showIcons.value)}
      />
      <SelectorButton label="Main Page Location">London</SelectorButton>
      <SelectorButton label="Temperature">Celsius</SelectorButton>
    </Screen>
  );
}
