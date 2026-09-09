import { resource, useSnapshot, type SnapshotSource } from "ink";
import { getAirQualityData, getWeatherData, type AirQualityData, type WeatherData } from "../lib/weather";
import type { Place } from "../lib/place";
import type { WeatherPreferences } from "../lib/preferences";

export interface ForecastResult {
  key: string | null;
  data: WeatherData | null;
  airQuality: AirQualityData | null;
  loading: boolean;
  error: string | null;
  retry: () => void;
}

const forecast = resource({
  key: (latitude: number, longitude: number, temperature: WeatherPreferences["temperatureUnit"], wind: WeatherPreferences["windSpeedUnit"], precipitation: WeatherPreferences["precipitationUnit"]) =>
    [latitude, longitude, temperature, wind, precipitation],
  load: async (latitude, longitude, temperature, wind, precipitation) => {
    const [data, airQuality] = await Promise.all([
      getWeatherData(latitude, longitude, temperature, wind, precipitation),
      getAirQualityData(latitude, longitude),
    ]);
    return { data, airQuality };
  },
  staleTime: 60_000,
  refreshInterval: 60_000,
});
const emptyState = { status: "ready", data: null } as const;
const empty: SnapshotSource<null> = { getSnapshot: () => emptyState, subscribe: () => () => {} };

export function useForecast(place: Place | null, prefs: WeatherPreferences): ForecastResult {
  const { temperatureUnit, windSpeedUnit, precipitationUnit } = prefs;
  const source = place ? forecast(place.latitude, place.longitude, temperatureUnit, windSpeedUnit, precipitationUnit) : null;
  const state = useSnapshot<{ data: WeatherData; airQuality: AirQualityData | null } | null>(source ?? empty);
  return {
    key: place ? `${place.key}:${temperatureUnit}:${windSpeedUnit}:${precipitationUnit}` : null,
    data: state.status === "ready" ? state.data?.data ?? null : null,
    airQuality: state.status === "ready" ? state.data?.airQuality ?? null : null,
    loading: state.status === "loading",
    error: state.status === "error" ? state.error.message : null,
    retry: () => { void source?.refresh(); },
  };
}
