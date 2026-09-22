import { Image, useColourScheme } from "ink";
import { getWeatherIconKey } from "../lib/weather";
import daySunnyLight from "../assets/weather/wi-day-sunny-black.png";
import daySunnyDark from "../assets/weather/wi-day-sunny-white.png";
import nightClearLight from "../assets/weather/wi-night-clear-black.png";
import nightClearDark from "../assets/weather/wi-night-clear-white.png";
import daySunnyOvercastLight from "../assets/weather/wi-day-sunny-overcast-black.png";
import daySunnyOvercastDark from "../assets/weather/wi-day-sunny-overcast-white.png";
import nightPartlyCloudyLight from "../assets/weather/wi-night-partly-cloudy-black.png";
import nightPartlyCloudyDark from "../assets/weather/wi-night-partly-cloudy-white.png";
import cloudLight from "../assets/weather/wi-cloud-black.png";
import cloudDark from "../assets/weather/wi-cloud-white.png";
import cloudyLight from "../assets/weather/wi-cloudy-black.png";
import cloudyDark from "../assets/weather/wi-cloudy-white.png";
import dayFogLight from "../assets/weather/wi-day-fog-black.png";
import dayFogDark from "../assets/weather/wi-day-fog-white.png";
import nightFogLight from "../assets/weather/wi-night-fog-black.png";
import nightFogDark from "../assets/weather/wi-night-fog-white.png";
import daySprinkleLight from "../assets/weather/wi-day-sprinkle-black.png";
import daySprinkleDark from "../assets/weather/wi-day-sprinkle-white.png";
import nightSprinkleLight from "../assets/weather/wi-night-sprinkle-black.png";
import nightSprinkleDark from "../assets/weather/wi-night-sprinkle-white.png";
import dayRainLight from "../assets/weather/wi-day-rain-black.png";
import dayRainDark from "../assets/weather/wi-day-rain-white.png";
import nightRainLight from "../assets/weather/wi-night-rain-black.png";
import nightRainDark from "../assets/weather/wi-night-rain-white.png";
import daySnowLight from "../assets/weather/wi-day-snow-black.png";
import daySnowDark from "../assets/weather/wi-day-snow-white.png";
import nightSnowLight from "../assets/weather/wi-night-snow-black.png";
import nightSnowDark from "../assets/weather/wi-night-snow-white.png";
import dayStormLight from "../assets/weather/wi-day-thunderstorm-black.png";
import dayStormDark from "../assets/weather/wi-day-thunderstorm-white.png";
import nightStormLight from "../assets/weather/wi-night-thunderstorm-black.png";
import nightStormDark from "../assets/weather/wi-night-thunderstorm-white.png";
import sunriseLight from "../assets/weather/wi-sunrise-black.png";
import sunriseDark from "../assets/weather/wi-sunrise-white.png";
import sunsetLight from "../assets/weather/wi-sunset-black.png";
import sunsetDark from "../assets/weather/wi-sunset-white.png";

const weatherImages = {
  sunny: { dark: daySunnyDark, light: daySunnyLight },
  clearNight: { dark: nightClearDark, light: nightClearLight },
  partlyCloudy: { dark: daySunnyOvercastDark, light: daySunnyOvercastLight },
  partlyCloudyNight: { dark: nightPartlyCloudyDark, light: nightPartlyCloudyLight },
  cloud: { dark: cloudDark, light: cloudLight },
  cloudy: { dark: cloudyDark, light: cloudyLight },
  fog: { dark: dayFogDark, light: dayFogLight },
  nightFog: { dark: nightFogDark, light: nightFogLight },
  drizzle: { dark: daySprinkleDark, light: daySprinkleLight },
  nightDrizzle: { dark: nightSprinkleDark, light: nightSprinkleLight },
  rain: { dark: dayRainDark, light: dayRainLight },
  nightRain: { dark: nightRainDark, light: nightRainLight },
  snow: { dark: daySnowDark, light: daySnowLight },
  nightSnow: { dark: nightSnowDark, light: nightSnowLight },
  storm: { dark: dayStormDark, light: dayStormLight },
  nightStorm: { dark: nightStormDark, light: nightStormLight },
  sunrise: { dark: sunriseDark, light: sunriseLight },
  sunset: { dark: sunsetDark, light: sunsetLight },
};

const imagesByName = new Map(Object.entries(weatherImages));

export function WeatherSymbol({
  code,
  isDay,
  kind,
  size,
}: {
  code?: number | null;
  isDay?: number | null;
  kind?: string;
  size: number;
}) {
  const scheme = useColourScheme();
  const key = kind ?? getWeatherIconKey(code ?? 3, isDay ?? 1);
  const source = imagesByName.get(key) ?? weatherImages.cloud;
  return <Image src={source[scheme]} width={size} height={size} fit="contain" />;
}
