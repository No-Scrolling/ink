import { useEffect, useState } from "react";
import { getAirQualityData, getWeatherData, type AirQualityData, type WeatherData } from "../lib/weather";
import type { Place } from "../lib/place";
import type { WeatherPreferences } from "../lib/preferences";

interface ForecastState {
  key: string | null;
  data: WeatherData | null;
  airQuality: AirQualityData | null;
  loading: boolean;
  error: string | null;
}

export type ForecastResult = ForecastState & { retry: () => void };

export function useForecast(place: Place | null, prefs: WeatherPreferences): ForecastResult {
  const [state, setState] = useState<ForecastState>({ key: null, data: null, airQuality: null, loading: false, error: null });
  const [attempt, setAttempt] = useState(0);
  const unitKey = `${prefs.temperatureUnit}:${prefs.windSpeedUnit}:${prefs.precipitationUnit}`;
  const requestKey = place ? `${place.key}:${unitKey}` : null;
  useEffect(() => {
    if (!place || !requestKey) {
      setState({ key: null, data: null, airQuality: null, loading: false, error: null });
      return;
    }
    let active = true;
    setState(current => current.key === requestKey
      ? { ...current, loading: true, error: null }
      : { key: requestKey, data: null, airQuality: null, loading: true, error: null });
    void Promise.all([
      getWeatherData(place.latitude, place.longitude, prefs.temperatureUnit, prefs.windSpeedUnit, prefs.precipitationUnit),
      getAirQualityData(place.latitude, place.longitude),
    ]).then(([data, airQuality]) => {
      if (active) setState({ key: requestKey, data, airQuality, loading: false, error: null });
    }).catch(cause => {
      if (active) setState(current => ({ ...current, key: requestKey, loading: false, error: cause instanceof Error ? cause.message : "Could not fetch weather." }));
    });
    const refresh = setInterval(() => setAttempt(value => value + 1), 60_000);

    return () => { active = false; clearInterval(refresh); };
  }, [attempt, place?.key, place?.latitude, place?.longitude, requestKey, unitKey]);
  return { ...state, retry: () => setAttempt(value => value + 1) };
}
