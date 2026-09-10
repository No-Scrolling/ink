import { ErrorState, LoadingState, Screen, Text } from "ink";
import { PreferencesGate } from "../../components/PreferencesGate";
import { Forecast } from "../../components/Forecast";
import { useCurrentPlace } from "../../hooks/useCurrentPlace";
import { weatherPlace } from "../../lib/place";
import type { WeatherPreferences } from "../../lib/preferences";

export default function CurrentWeatherScreen() {
  return <PreferencesGate title="Weather">{prefs => prefs.mainLocation
    ? <Forecast place={weatherPlace(prefs.mainLocation)} prefs={prefs} />
    : <CurrentWeather prefs={prefs} />}</PreferencesGate>;
}

function CurrentWeather({ prefs }: { prefs: WeatherPreferences }) {
  const currentPlace = useCurrentPlace();
  if (currentPlace.loading && !currentPlace.place) return <Screen title="Weather"><LoadingState label="Finding your location…" /></Screen>;
  if (currentPlace.error && !currentPlace.place) {
    return <Screen title="Weather"><ErrorState message={currentPlace.error} onRetry={currentPlace.retry} /></Screen>;
  }
  if (!currentPlace.place) return <Screen title="Weather"><Text align="center">Choose a location in Locations or Settings.</Text></Screen>;
  return <Forecast place={currentPlace.place} prefs={prefs} notice={currentPlace.error ?? undefined} onRetryNotice={currentPlace.retry} />;
}
