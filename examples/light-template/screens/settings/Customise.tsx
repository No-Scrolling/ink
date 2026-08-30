import { Button, Screen, Stack, Text, persistedState } from "ink";

export default function Customise() {
  const temperatureUnit = persistedState("settings.temperature-unit", "Celsius");

  return (
    <Screen title="Customise">
      <Button href="/settings/customise-interface">Interface</Button>
      <Stack gap={0}>
        <Text size={20}>Temperature</Text>
        <Button href="/settings/temperature-unit">{temperatureUnit.value}</Button>
      </Stack>
    </Screen>
  );
}
