import { ErrorState, Screen, Text, useRouteParams } from "ink";
import { useMemo } from "react";
import { star, starFilled } from "ink/icons";
import { PreferencesGate } from "../components/PreferencesGate";
import { Forecast } from "../components/Forecast";
import { useSettingsUpdate } from "../hooks/useSettingsUpdate";
import { weatherPlace } from "../lib/place";
import { decodeWeatherRoute } from "../lib/routeParams";
import type { WeatherPreferences } from "../lib/preferences";

export default function SearchWeatherScreen() {
  return <PreferencesGate title="Weather">{prefs => <SearchWeather prefs={prefs} />}</PreferencesGate>;
}

function SearchWeather({ prefs }: { prefs: WeatherPreferences }) {
  const { location } = useRouteParams(decodeWeatherRoute);
  const place = useMemo(() => location ? weatherPlace(location) : null, [location?.id, location?.latitude, location?.longitude, location?.name, location?.country, location?.admin1]);
  const isSaved = location !== null && prefs.savedLocations.some(saved => saved.id === location.id && saved.latitude === location.latitude && saved.longitude === location.longitude);
  const save = useSettingsUpdate();
  if (!place) return <Screen title="Weather"><Text>Invalid location coordinates.</Text></Screen>;
  const toggleSaved = () => {
    if (!location) return;
    const savedLocations = isSaved
      ? prefs.savedLocations.filter(saved => !(saved.id === location.id && saved.latitude === location.latitude && saved.longitude === location.longitude))
      : [...prefs.savedLocations, location];
    save.run({ savedLocations });
  };
  if (save.status === "error") return <Screen title={place.label}><ErrorState message="Could not save this location." onRetry={save.retry} /></Screen>;
  return <Forecast
    place={place}
    prefs={prefs}
    rightAction={{ icon: isSaved ? starFilled : star, onPress: toggleSaved }}
  />;
}
