import { resource, useSnapshot } from "ink";
import { getAirQualityData, getWeatherData } from "../lib/weather";
import type { Place } from "../lib/place";
import type { WeatherPreferences } from "../lib/preferences";

const forecast = resource({
  key: (
    latitude: number,
    longitude: number,
    temperature: WeatherPreferences["temperatureUnit"],
    wind: WeatherPreferences["windSpeedUnit"],
    precipitation: WeatherPreferences["precipitationUnit"],
  ) => [latitude, longitude, temperature, wind, precipitation],
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

export function useForecast(place: Place, prefs: WeatherPreferences) {
  const { temperatureUnit, windSpeedUnit, precipitationUnit } = prefs;
  const source = forecast(
    place.latitude,
    place.longitude,
    temperatureUnit,
    windSpeedUnit,
    precipitationUnit,
  );
  const state = useSnapshot(source);
  return {
    data: state.status === "ready" ? state.data.data : null,
    airQuality: state.status === "ready" ? state.data.airQuality : null,
    loading: state.status === "loading",
    error:
      state.status === "error"
        ? state.error.message
        : state.status === "ready"
          ? (state.refreshError?.message ?? null)
          : null,
    retry: source.refresh,
  };
}
