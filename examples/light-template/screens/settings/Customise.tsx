import { Button, Screen, SelectorButton, persistedState } from "ink";

export default function Customise() {
  const temperatureUnit = persistedState("settings.temperature-unit", "Celsius");

  return (
    <Screen title="Customise">
      <Button href="/settings/customise-interface">Interface</Button>
      <SelectorButton label="Temperature" href="/settings/temperature-unit">
        {temperatureUnit.value}
      </SelectorButton>
    </Screen>
  );
}
