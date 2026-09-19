import { useMemo } from "react";
import { Button, ErrorState, List, LoadingState, Screen, Stack, Text, type IconAsset } from "ink";
import { formatNumber, formatTime, formatWeekday, getWeatherDescription, type AirQualityData, type WeatherData } from "../lib/weather";
import { DETAIL_LABELS, type WeatherDetail, type WeatherPreferences } from "../lib/preferences";
import type { Place } from "../lib/place";
import { useForecast } from "../hooks/useForecast";
import { WeatherSymbol } from "./WeatherSymbol";

function qualityValue(airQuality: AirQualityData | null, time: string, key: "usAqi" | "europeanAqi" | "pm25" | "pm10"): number | null {
  if (!airQuality) return null;
  const index = airQuality.hourly.time.indexOf(time);
  return index < 0 ? null : airQuality.hourly[key][index] ?? null;
}

function dailyQualityValue(airQuality: AirQualityData | null, date: string, key: "usAqi" | "europeanAqi" | "pm25" | "pm10"): number | null {
  if (!airQuality) return null;
  let maximum: number | null = null;
  const dateKey = date.slice(0, 10);
  for (let index = 0; index < airQuality.hourly.time.length; index++) {
    if (!airQuality.hourly.time[index].startsWith(dateKey)) continue;
    const value = airQuality.hourly[key][index];
    if (value !== null && (maximum === null || value > maximum)) maximum = value;
  }
  return maximum;
}

function hourlyDetail(
  detail: WeatherDetail,
  data: WeatherData,
  index: number,
  prefs: WeatherPreferences,
  airQuality: AirQualityData | null,
): string {
  const hourly = data.hourly;
  switch (detail) {
    case "Temp": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.temperature[index])}°`;
    case "Feels Like": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.apparentTemperature[index])}°`;
    case "Precip Chance": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.precipitationProbability[index])}%`;
    case "Precip Amount": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.precipitation[index], 2)}${prefs.precipitationUnit === "Millimeter" ? "mm" : "in"}`;
    case "Wind Speed": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.windSpeed[index])} ${prefs.windSpeedUnit}`;
    case "Wind Gusts": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.windGusts[index])} ${prefs.windSpeedUnit}`;
    case "UV Index": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.uvIndex[index], 1)}`;
    case "Humidity": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.humidity[index])}%`;
    case "Dew Point": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.dewPoint[index])}°`;
    case "Cloud Cover": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.cloudCover[index])}%`;
    case "Visibility": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.visibility[index] === null ? null : hourly.visibility[index] / 1000, 1)} km`;
    case "Pressure": return `${DETAIL_LABELS[detail]}: ${formatNumber(hourly.pressure[index])} hPa`;
    case "AQI (US)": return `${DETAIL_LABELS[detail]}: ${formatNumber(qualityValue(airQuality, hourly.time[index], "usAqi"))}`;
    case "AQI (EU)": return `${DETAIL_LABELS[detail]}: ${formatNumber(qualityValue(airQuality, hourly.time[index], "europeanAqi"))}`;
    case "PM2.5": return `${DETAIL_LABELS[detail]}: ${formatNumber(qualityValue(airQuality, hourly.time[index], "pm25"))} μg/m³`;
    case "PM10": return `${DETAIL_LABELS[detail]}: ${formatNumber(qualityValue(airQuality, hourly.time[index], "pm10"))} μg/m³`;
  }
}

function dailyDetail(
  detail: WeatherDetail,
  data: WeatherData,
  index: number,
  prefs: WeatherPreferences,
  airQuality: AirQualityData | null,
): string {
  const daily = data.daily;
  switch (detail) {
    case "Temp": return `${DETAIL_LABELS[detail]}: (${formatNumber(daily.temperatureMin[index])}°, ${formatNumber(daily.temperatureMax[index])}°)`;
    case "Feels Like": return `${DETAIL_LABELS[detail]}: (${formatNumber(daily.apparentTemperatureMin[index])}°, ${formatNumber(daily.apparentTemperatureMax[index])}°)`;
    case "Precip Chance": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.precipitationProbability[index])}%`;
    case "Precip Amount": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.precipitation[index], 2)}${prefs.precipitationUnit === "Millimeter" ? "mm" : "in"}`;
    case "Wind Speed": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.windSpeed[index])} ${prefs.windSpeedUnit}`;
    case "Wind Gusts": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.windGusts[index])} ${prefs.windSpeedUnit}`;
    case "UV Index": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.uvIndex[index], 1)}`;
    case "Humidity": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.humidity[index])}%`;
    case "Dew Point": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.dewPoint[index])}°`;
    case "Cloud Cover": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.cloudCover[index])}%`;
    case "Visibility": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.visibility[index] === null ? null : daily.visibility[index] / 1000, 1)} km`;
    case "Pressure": return `${DETAIL_LABELS[detail]}: ${formatNumber(daily.pressure[index])} hPa`;
    case "AQI (US)": return `${DETAIL_LABELS[detail]}: ${formatNumber(dailyQualityValue(airQuality, daily.time[index], "usAqi"))}`;
    case "AQI (EU)": return `${DETAIL_LABELS[detail]}: ${formatNumber(dailyQualityValue(airQuality, daily.time[index], "europeanAqi"))}`;
    case "PM2.5": return `${DETAIL_LABELS[detail]}: ${formatNumber(dailyQualityValue(airQuality, daily.time[index], "pm25"))} μg/m³`;
    case "PM10": return `${DETAIL_LABELS[detail]}: ${formatNumber(dailyQualityValue(airQuality, daily.time[index], "pm10"))} μg/m³`;
  }
}

function DetailsLines({ values }: { values: string[] }) {
  return <Stack gap={0}>{values.map(value => <Text key={value} size={16}>{value}</Text>)}</Stack>;
}

export function Forecast({
  place,
  prefs,
  rightAction,
  notice,
  onRetryNotice,
}: {
  place: Place;
  prefs: WeatherPreferences;
  rightAction?: { icon: IconAsset; onPress: () => void };
  notice?: string;
  onRetryNotice?: () => void;
}) {
  const state = useForecast(place, prefs);
  const title = place.label;
  const data = state.data;
  const rows = useMemo(() => {
    if (!data) return [];
    const hours = data.hourly.time.slice(0, 24);
    const rows: Array<{ time: string; index: number; event?: "sunrise" | "sunset" }> = hours.map((time, index) => ({ time, index }));
    for (const event of ["sunrise", "sunset"] as const) {
      for (const time of data.daily[event]) {
        if (time >= hours[0] && time.slice(0, 13) <= hours[hours.length - 1]?.slice(0, 13)) {
          rows.push({ time, index: -1, event });
        }
      }
    }
    rows.sort((a, b) => a.time.localeCompare(b.time));
    return rows;
  }, [data]);
  if (state.loading) return <Screen title={title}><LoadingState label="Loading weather…" /></Screen>;
  if (!data && state.error) return <Screen title={title}><ErrorState message={state.error} onRetry={state.retry} /></Screen>;
  if (!data) return <Screen title={title} rightAction={rightAction}><Text align="center">Weather is unavailable.</Text></Screen>;

  const current = data.current;
  const details = prefs.selectedDetails;
  return (
    <Screen title={title} rightAction={rightAction}>
      <Stack gap={24}>
        <Stack align="center" gap={0}>
          <Stack axis="horizontal" align="center" justify="center" gap={8}>
            <WeatherSymbol code={current.weatherCode} isDay={current.isDay} size={100} />
            <Text size={88}>{formatNumber(current.temperature)}°</Text>
          </Stack>
          <Text size={20}>
            Feels like {formatNumber(current.apparentTemperature)}°, L: {formatNumber(data.daily.temperatureMin[0])}° H: {formatNumber(data.daily.temperatureMax[0])}°
          </Text>
          {state.error && <Text size={16}>Weather may be out of date.</Text>}
          {state.error && <Button onPress={state.retry}>Try again</Button>}
          {notice && <Text size={16}>{notice}</Text>}
          {notice && onRetryNotice && <Button onPress={onRetryNotice}>Try location again</Button>}
        </Stack>
        <Stack gap={0}>
          <Text size={16}>Hourly Forecast</Text>
          <List
            items={rows}
            keyExtractor={row => `${row.time}:${row.event ?? "weather"}`}
            gap={14}
            renderItem={row => row.event ? (
              <Stack axis="horizontal" align="center" gap={8}>
                <Text size={26} width={prefs.timeFormat === "12h" ? 140 : 84}>{formatTime(row.time, prefs.timeFormat)}</Text>
                <WeatherSymbol kind={row.event} size={32} />
                <Text size={26}>{row.event === "sunrise" ? "Sunrise" : "Sunset"}</Text>
              </Stack>
            ) : (
              <HourlyRow data={data} prefs={prefs} airQuality={state.airQuality} details={details} index={row.index} />
            )}
          />
        </Stack>
        <Stack gap={0}>
          <Text size={16}>Weekly Forecast</Text>
          <List
            items={data.daily.time}
            keyExtractor={time => time}
            gap={14}
            renderItem={(_, index) => (
              <Stack axis="horizontal" align="start" gap={8}>
                <Text size={26} width={prefs.timeFormat === "12h" ? 140 : 84}>{formatWeekday(data.daily.time[index])}</Text>
                <Stack gap={0}>
                  <Stack axis="horizontal" align="center" gap={8}>
                    <WeatherSymbol code={data.daily.weatherCode[index]} isDay={1} size={32} />
                    <Text size={26}>{getWeatherDescription(data.daily.weatherCode[index] ?? 3)}</Text>
                  </Stack>
                  <DetailsLines values={details.map(detail => dailyDetail(detail, data, index, prefs, state.airQuality))} />
                </Stack>
              </Stack>
            )}
          />
        </Stack>
      </Stack>
    </Screen>
  );
}

function HourlyRow({
  data,
  prefs,
  airQuality,
  details,
  index,
}: {
  data: WeatherData;
  prefs: WeatherPreferences;
  airQuality: AirQualityData | null;
  details: WeatherDetail[];
  index: number;
}) {
  return (
    <Stack axis="horizontal" align="start" gap={8}>
      <Text size={26} width={prefs.timeFormat === "12h" ? 140 : 84}>{formatTime(data.hourly.time[index], prefs.timeFormat)}</Text>
      <Stack gap={0}>
        <Stack axis="horizontal" align="center" gap={8}>
          <WeatherSymbol code={data.hourly.weatherCode[index]} isDay={data.hourly.isDay[index]} size={32} />
          <Text size={26}>{getWeatherDescription(data.hourly.weatherCode[index] ?? 3)}</Text>
        </Stack>
        <DetailsLines values={details.map(detail => hourlyDetail(detail, data, index, prefs, airQuality))} />
      </Stack>
    </Stack>
  );
}
