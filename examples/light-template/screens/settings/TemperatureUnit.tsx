import { Button, Screen, back, persistedState } from "ink";

export default function TemperatureUnit() {
  const temperatureUnit = persistedState("settings.temperature-unit", "Celsius");

  return (
    <Screen title="Temperature Unit">
      <Button
        underline={temperatureUnit.value === "Celsius"}
        onPress={() => {
          temperatureUnit.set("Celsius");
          back();
        }}
      >
        Celsius
      </Button>
      <Button
        underline={temperatureUnit.value === "Fahrenheit"}
        onPress={() => {
          temperatureUnit.set("Fahrenheit");
          back();
        }}
      >
        Fahrenheit
      </Button>
    </Screen>
  );
}
