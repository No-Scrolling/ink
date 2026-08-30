import { json } from "@ink/network";
import { Button, Icon, Screen, Stack, Text } from "ink";

type Forecast = {
  current: {
    temperature_2m: number;
    apparent_temperature: number;
    weather_code: number;
  };
  daily: {
    time: string[];
    temperature_2m_max: number[];
    temperature_2m_min: number[];
  };
};

export default function Weather() {
  const forecast = json<Forecast>("https://api.open-meteo.com/v1/forecast", {
    query: {
      latitude: 51.5072,
      longitude: -0.1276,
      current: "temperature_2m,apparent_temperature,weather_code",
      daily: "temperature_2m_max,temperature_2m_min",
      timezone: "Europe/London",
      forecast_days: 7,
    },
  });

  return (
    <Screen title="London">
      {forecast.status === "loading" ? (
        <Text size={18} align="center">Loading weather...</Text>
      ) : forecast.status === "ready" ? (
        <Stack gap={47}>
          <Stack axis="horizontal" gap={4} align="center" justify="center">
            <Icon name="partly_cloudy_day" size={100} />
            <Text size={88}>{forecast.value.current.temperature_2m}°</Text>
          </Stack>
          <Text size={20} align="center">
            Feels like {forecast.value.current.apparent_temperature}°
          </Text>
          <Stack gap={15}>
            <Text size={20}>Seven Day Forecast</Text>
            {forecast.value.daily.time.map((day) => (
              <Text>{day}</Text>
            ))}
          </Stack>
          <Stack axis="horizontal" gap={47} justify="center">
            <Stack gap={15} align="center">
              <Text size={20}>Low</Text>
              {forecast.value.daily.temperature_2m_min.map((temperature) => (
                <Text>{temperature}°</Text>
              ))}
            </Stack>
            <Stack gap={15} align="center">
              <Text size={20}>High</Text>
              {forecast.value.daily.temperature_2m_max.map((temperature) => (
                <Text>{temperature}°</Text>
              ))}
            </Stack>
          </Stack>
        </Stack>
      ) : (
        <Stack gap={47}>
          <Text size={18} align="center">{forecast.error.message}</Text>
          <Button onPress={() => forecast.reload()}>Try Again</Button>
        </Stack>
      )}
    </Screen>
  );
}
