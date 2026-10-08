import * as v from "valibot";
import "@ink/network";

import type { PrecipitationUnit, TemperatureUnit, WindSpeedUnit } from "./preferences";

export interface WeatherData {
  current: {
    time: string;
    weatherCode: number;
    temperature: number;
    apparentTemperature: number;
    isDay: number;
  };
  hourly: {
    time: string[];
    temperature: Array<number | null>;
    apparentTemperature: Array<number | null>;
    precipitationProbability: Array<number | null>;
    precipitation: Array<number | null>;
    weatherCode: Array<number | null>;
    windSpeed: Array<number | null>;
    windGusts: Array<number | null>;
    uvIndex: Array<number | null>;
    humidity: Array<number | null>;
    dewPoint: Array<number | null>;
    cloudCover: Array<number | null>;
    visibility: Array<number | null>;
    pressure: Array<number | null>;
    isDay: Array<number | null>;
  };
  daily: {
    time: string[];
    temperatureMax: Array<number | null>;
    temperatureMin: Array<number | null>;
    weatherCode: Array<number | null>;
    apparentTemperatureMax: Array<number | null>;
    apparentTemperatureMin: Array<number | null>;
    precipitationProbability: Array<number | null>;
    uvIndex: Array<number | null>;
    precipitation: Array<number | null>;
    windSpeed: Array<number | null>;
    windGusts: Array<number | null>;
    humidity: Array<number | null>;
    dewPoint: Array<number | null>;
    cloudCover: Array<number | null>;
    visibility: Array<number | null>;
    pressure: Array<number | null>;
    sunrise: string[];
    sunset: string[];
  };
}

export interface AirQualityData {
  hourly: {
    time: string[];
    usAqi: Array<number | null>;
    europeanAqi: Array<number | null>;
    pm25: Array<number | null>;
    pm10: Array<number | null>;
  };
}

const finite = v.pipe(v.number(), v.finite());
const numbers = v.array(v.nullable(finite));
const currentSchema = v.pipe(
  v.object({
    time: v.string(),
    weather_code: finite,
    temperature_2m: finite,
    apparent_temperature: finite,
    is_day: finite,
  }),
  v.transform((value) => ({
    time: value.time,
    weatherCode: value.weather_code,
    temperature: value.temperature_2m,
    apparentTemperature: value.apparent_temperature,
    isDay: value.is_day,
  })),
);
const hourlySchema = v.pipe(
  v.object({
    time: v.array(v.string()),
    temperature_2m: numbers,
    apparent_temperature: numbers,
    precipitation_probability: numbers,
    precipitation: numbers,
    weather_code: numbers,
    wind_speed_10m: numbers,
    wind_gusts_10m: numbers,
    uv_index: numbers,
    relative_humidity_2m: numbers,
    dew_point_2m: numbers,
    cloud_cover: numbers,
    visibility: numbers,
    surface_pressure: numbers,
    is_day: numbers,
  }),
  v.transform((value) => ({
    time: value.time,
    temperature: value.temperature_2m,
    apparentTemperature: value.apparent_temperature,
    precipitationProbability: value.precipitation_probability,
    precipitation: value.precipitation,
    weatherCode: value.weather_code,
    windSpeed: value.wind_speed_10m,
    windGusts: value.wind_gusts_10m,
    uvIndex: value.uv_index,
    humidity: value.relative_humidity_2m,
    dewPoint: value.dew_point_2m,
    cloudCover: value.cloud_cover,
    visibility: value.visibility,
    pressure: value.surface_pressure,
    isDay: value.is_day,
  })),
);
const dailySchema = v.pipe(
  v.object({
    time: v.array(v.string()),
    temperature_2m_max: numbers,
    temperature_2m_min: numbers,
    weather_code: numbers,
    apparent_temperature_max: numbers,
    apparent_temperature_min: numbers,
    precipitation_probability_max: numbers,
    uv_index_max: numbers,
    precipitation_sum: numbers,
    wind_speed_10m_max: numbers,
    wind_gusts_10m_max: numbers,
    relative_humidity_2m_mean: numbers,
    dew_point_2m_mean: numbers,
    cloud_cover_mean: numbers,
    visibility_mean: numbers,
    surface_pressure_mean: numbers,
    sunrise: v.array(v.string()),
    sunset: v.array(v.string()),
  }),
  v.transform((value) => ({
    time: value.time,
    temperatureMax: value.temperature_2m_max,
    temperatureMin: value.temperature_2m_min,
    weatherCode: value.weather_code,
    apparentTemperatureMax: value.apparent_temperature_max,
    apparentTemperatureMin: value.apparent_temperature_min,
    precipitationProbability: value.precipitation_probability_max,
    uvIndex: value.uv_index_max,
    precipitation: value.precipitation_sum,
    windSpeed: value.wind_speed_10m_max,
    windGusts: value.wind_gusts_10m_max,
    humidity: value.relative_humidity_2m_mean,
    dewPoint: value.dew_point_2m_mean,
    cloudCover: value.cloud_cover_mean,
    visibility: value.visibility_mean,
    pressure: value.surface_pressure_mean,
    sunrise: value.sunrise,
    sunset: value.sunset,
  })),
);
const weatherSchema = v.object({
  current: currentSchema,
  hourly: hourlySchema,
  daily: dailySchema,
});
const airSchema = v.object({
  hourly: v.pipe(
    v.object({
      time: v.array(v.string()),
      us_aqi: numbers,
      european_aqi: numbers,
      pm2_5: numbers,
      pm10: numbers,
    }),
    v.transform((value) => ({
      time: value.time,
      usAqi: value.us_aqi,
      europeanAqi: value.european_aqi,
      pm25: value.pm2_5,
      pm10: value.pm10,
    })),
  ),
});
const geocodingSchema = v.object({
  results: v.optional(
    v.array(
      v.fallback(
        v.nullable(
          v.object({
            id: v.number(),
            name: v.string(),
            latitude: v.number(),
            longitude: v.number(),
            country: v.string(),
            country_code: v.string(),
            admin1: v.fallback(v.optional(v.string()), undefined),
          }),
        ),
        null,
      ),
    ),
    [],
  ),
});
async function requestJson(url: string): Promise<string> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 20_000);
  try {
    const response = await fetch(url, { signal: controller.signal });
    if (!response.ok) throw new Error(`Request failed (HTTP ${response.status}).`);
    return await response.text();
  } finally {
    clearTimeout(timeout);
  }
}

function unitValues(
  temperatureUnit: TemperatureUnit,
  windSpeedUnit: WindSpeedUnit,
  precipitationUnit: PrecipitationUnit,
) {
  return {
    temperature_unit: temperatureUnit === "Celsius" ? "celsius" : "fahrenheit",
    wind_speed_unit:
      windSpeedUnit === "km/h"
        ? "kmh"
        : windSpeedUnit === "m/s"
          ? "ms"
          : windSpeedUnit === "Knots"
            ? "kn"
            : "mph",
    precipitation_unit: precipitationUnit === "Millimeter" ? "mm" : "inch",
  };
}

export async function getWeatherData(
  latitude: number,
  longitude: number,
  temperatureUnit: TemperatureUnit,
  windSpeedUnit: WindSpeedUnit,
  precipitationUnit: PrecipitationUnit,
): Promise<WeatherData> {
  const url = new URL("https://api.open-meteo.com/v1/forecast");
  url.searchParams.set("latitude", String(latitude));
  url.searchParams.set("longitude", String(longitude));
  url.searchParams.set("timezone", "auto");
  url.searchParams.set("forecast_days", "7");
  url.searchParams.set("forecast_hours", "24");
  url.searchParams.set("current", "weather_code,temperature_2m,apparent_temperature,is_day");
  url.searchParams.set(
    "hourly",
    [
      "temperature_2m",
      "apparent_temperature",
      "precipitation_probability",
      "precipitation",
      "weather_code",
      "wind_speed_10m",
      "wind_gusts_10m",
      "uv_index",
      "relative_humidity_2m",
      "dew_point_2m",
      "cloud_cover",
      "visibility",
      "surface_pressure",
      "is_day",
    ].join(","),
  );
  url.searchParams.set(
    "daily",
    [
      "temperature_2m_max",
      "temperature_2m_min",
      "weather_code",
      "apparent_temperature_max",
      "apparent_temperature_min",
      "precipitation_probability_max",
      "uv_index_max",
      "precipitation_sum",
      "wind_speed_10m_max",
      "wind_gusts_10m_max",
      "relative_humidity_2m_mean",
      "dew_point_2m_mean",
      "cloud_cover_mean",
      "visibility_mean",
      "surface_pressure_mean",
      "sunrise",
      "sunset",
    ].join(","),
  );
  const units = unitValues(temperatureUnit, windSpeedUnit, precipitationUnit);
  for (const [key, value] of Object.entries(units)) url.searchParams.set(key, value);
  return v.parse(weatherSchema, JSON.parse(await requestJson(url.href)));
}

export async function getAirQualityData(
  latitude: number,
  longitude: number,
): Promise<AirQualityData | null> {
  const url = new URL("https://air-quality-api.open-meteo.com/v1/air-quality");
  url.searchParams.set("latitude", String(latitude));
  url.searchParams.set("longitude", String(longitude));
  url.searchParams.set("timezone", "auto");
  url.searchParams.set("forecast_days", "7");
  url.searchParams.set("hourly", "us_aqi,european_aqi,pm2_5,pm10");
  try {
    return v.parse(airSchema, JSON.parse(await requestJson(url.href)));
  } catch {
    return null;
  }
}

export interface GeocodingResult {
  id: number;
  name: string;
  latitude: number;
  longitude: number;
  country: string;
  countryCode: string;
  admin1?: string;
}

export async function searchLocations(query: string): Promise<GeocodingResult[]> {
  const url = new URL("https://geocoding-api.open-meteo.com/v1/search");
  url.searchParams.set("name", query);
  url.searchParams.set("count", "10");
  url.searchParams.set("language", "en");
  url.searchParams.set("format", "json");
  const body = v.parse(geocodingSchema, JSON.parse(await requestJson(url.href)));
  return body.results.flatMap((result) =>
    result
      ? [
          {
            id: result.id,
            name: result.name,
            latitude: result.latitude,
            longitude: result.longitude,
            country: result.country,
            countryCode: result.country_code,
            admin1: result.admin1,
          },
        ]
      : [],
  );
}

export function formatNumber(value: number | null | undefined, decimals = 0): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "—";
  const multiplier = 10 ** decimals;
  const rounded = Math.round(value * multiplier) / multiplier;
  return decimals === 0 ? String(rounded === 0 ? 0 : rounded) : rounded.toFixed(decimals);
}

export function formatTime(value: string, format: "24h" | "12h"): string {
  const match = /^\d{4}-\d{2}-\d{2}T(\d{2}):(\d{2})/.exec(value);
  if (!match) return "—";
  const hour = Number(match[1]);
  const minute = match[2];
  if (format === "24h") return `${String(hour).padStart(2, "0")}:${minute}`;
  const suffix = hour >= 12 ? "PM" : "AM";
  const twelveHour = hour % 12 || 12;
  return `${twelveHour}:${minute} ${suffix}`;
}

export function formatWeekday(value: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(value);
  if (!match) return "—";
  const day = new Date(
    Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3])),
  ).getUTCDay();
  return ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][day];
}

export function getWeatherDescription(code: number): string {
  if (code === 0) return "Clear sky";
  if (code === 1) return "Mainly clear";
  if (code === 2) return "Partly cloudy";
  if (code === 3) return "Overcast";
  if (code === 45 || code === 48) return "Fog";
  if (code >= 51 && code <= 57) return code <= 53 ? "Drizzle" : "Freezing drizzle";
  if (code >= 61 && code <= 67) return code <= 65 ? "Rain" : "Freezing rain";
  if (code >= 71 && code <= 77) return "Snow";
  if (code >= 80 && code <= 82) return "Rain showers";
  if (code === 85 || code === 86) return "Snow showers";
  if (code >= 95) return "Thunderstorm";
  return "Unknown weather";
}

export function getWeatherIconKey(code: number, isDay: number): string {
  if (code === 0) return isDay === 1 ? "sunny" : "clearNight";
  if (code === 1 || code === 2) return isDay === 1 ? "partlyCloudy" : "partlyCloudyNight";
  if (code === 3) return "cloudy";
  if (code === 45 || code === 48) return isDay === 1 ? "fog" : "nightFog";
  if (code >= 51 && code <= 57) return isDay === 1 ? "drizzle" : "nightDrizzle";
  if ((code >= 61 && code <= 67) || (code >= 80 && code <= 82))
    return isDay === 1 ? "rain" : "nightRain";
  if ((code >= 71 && code <= 77) || code === 85 || code === 86)
    return isDay === 1 ? "snow" : "nightSnow";
  if (code >= 95) return isDay === 1 ? "storm" : "nightStorm";
  return "cloud";
}
