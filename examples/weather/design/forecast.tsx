import { Forecast } from "../components/Forecast";
import type { Place } from "../lib/place";
import type { WeatherPreferences } from "../lib/preferences";

export default function ForecastPreview({
  place,
  prefs,
}: {
  place: Place;
  prefs: WeatherPreferences;
}) {
  return <Forecast place={place} prefs={prefs} />;
}
