import "@ink/network";

import type {
  PrecipitationUnit,
  TemperatureUnit,
  WindSpeedUnit,
} from "./preferences";

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

type JsonRecord = Record<string, unknown>;

function record(value: unknown, label: string): JsonRecord {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`Invalid ${label} response.`);
  }
  return value as JsonRecord;
}

function finiteNumber(value: unknown, label: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`Invalid ${label} value.`);
  }
  return value;
}

function numberArray(value: unknown, label: string): Array<number | null> {
  if (!Array.isArray(value)) throw new Error(`Invalid ${label} data.`);
  return value.map((item, index) => {
    if (item === null) return null;
    return finiteNumber(item, `${label}[${index}]`);
  });
}

function stringArray(value: unknown, label: string): string[] {
  if (!Array.isArray(value)) throw new Error(`Invalid ${label} data.`);
  return value.map((item, index) => {
    if (typeof item !== "string") throw new Error(`Invalid ${label}[${index}] value.`);
    return item;
  });
}

function readCurrent(response: JsonRecord): WeatherData["current"] {
  const current = record(response.current, "current weather");
  const time = current.time;
  if (typeof time !== "string") throw new Error("Invalid current weather time.");
  return {
    time,
    weatherCode: finiteNumber(current.weather_code, "weather code"),
    temperature: finiteNumber(current.temperature_2m, "temperature"),
    apparentTemperature: finiteNumber(current.apparent_temperature, "feels like"),
    isDay: finiteNumber(current.is_day, "day state"),
  };
}

function readHourly(response: JsonRecord): WeatherData["hourly"] {
  const hourly = record(response.hourly, "hourly weather");
  return {
    time: stringArray(hourly.time, "hourly time"),
    temperature: numberArray(hourly.temperature_2m, "hourly temperature"),
    apparentTemperature: numberArray(hourly.apparent_temperature, "hourly feels like"),
    precipitationProbability: numberArray(hourly.precipitation_probability, "precipitation chance"),
    precipitation: numberArray(hourly.precipitation, "precipitation"),
    weatherCode: numberArray(hourly.weather_code, "hourly weather code"),
    windSpeed: numberArray(hourly.wind_speed_10m, "wind speed"),
    windGusts: numberArray(hourly.wind_gusts_10m, "wind gusts"),
    uvIndex: numberArray(hourly.uv_index, "UV index"),
    humidity: numberArray(hourly.relative_humidity_2m, "humidity"),
    dewPoint: numberArray(hourly.dew_point_2m, "dew point"),
    cloudCover: numberArray(hourly.cloud_cover, "cloud cover"),
    visibility: numberArray(hourly.visibility, "visibility"),
    pressure: numberArray(hourly.surface_pressure, "pressure"),
    isDay: numberArray(hourly.is_day, "day state"),
  };
}

function readDaily(response: JsonRecord): WeatherData["daily"] {
  const daily = record(response.daily, "daily weather");
  return {
    time: stringArray(daily.time, "daily time"),
    temperatureMax: numberArray(daily.temperature_2m_max, "maximum temperature"),
    temperatureMin: numberArray(daily.temperature_2m_min, "minimum temperature"),
    weatherCode: numberArray(daily.weather_code, "daily weather code"),
    apparentTemperatureMax: numberArray(daily.apparent_temperature_max, "maximum feels like"),
    apparentTemperatureMin: numberArray(daily.apparent_temperature_min, "minimum feels like"),
    precipitationProbability: numberArray(daily.precipitation_probability_max, "daily precipitation chance"),
    uvIndex: numberArray(daily.uv_index_max, "daily UV index"),
    precipitation: numberArray(daily.precipitation_sum, "daily precipitation"),
    windSpeed: numberArray(daily.wind_speed_10m_max, "daily wind speed"),
    windGusts: numberArray(daily.wind_gusts_10m_max, "daily wind gusts"),
    humidity: numberArray(daily.relative_humidity_2m_mean, "daily humidity"),
    dewPoint: numberArray(daily.dew_point_2m_mean, "daily dew point"),
    cloudCover: numberArray(daily.cloud_cover_mean, "daily cloud cover"),
    visibility: numberArray(daily.visibility_mean, "daily visibility"),
    pressure: numberArray(daily.surface_pressure_mean, "daily pressure"),
    sunrise: stringArray(daily.sunrise, "sunrise"),
    sunset: stringArray(daily.sunset, "sunset"),
  };
}

async function requestJson(url: string): Promise<{ response: Response; body: unknown }> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 20_000);
  try {
    const response = await fetch(url, { signal: controller.signal });
    const body = await response.json();
    return { response, body };
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
    wind_speed_unit: windSpeedUnit === "km/h" ? "kmh" : windSpeedUnit === "m/s" ? "ms" : windSpeedUnit === "Knots" ? "kn" : "mph",
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
  url.searchParams.set("hourly", [
    "temperature_2m", "apparent_temperature", "precipitation_probability", "precipitation",
    "weather_code", "wind_speed_10m", "wind_gusts_10m", "uv_index",
    "relative_humidity_2m", "dew_point_2m", "cloud_cover", "visibility", "surface_pressure", "is_day",
  ].join(","));
  url.searchParams.set("daily", [
    "temperature_2m_max", "temperature_2m_min", "weather_code", "apparent_temperature_max",
    "apparent_temperature_min", "precipitation_probability_max", "uv_index_max", "precipitation_sum",
    "wind_speed_10m_max", "wind_gusts_10m_max", "relative_humidity_2m_mean",
    "dew_point_2m_mean", "cloud_cover_mean", "visibility_mean", "surface_pressure_mean", "sunrise", "sunset",
  ].join(","));
  const units = unitValues(temperatureUnit, windSpeedUnit, precipitationUnit);
  for (const [key, value] of Object.entries(units)) url.searchParams.set(key, value);
  const { response, body: rawBody } = await requestJson(url.href);
  if (!response.ok) throw new Error(`Weather unavailable (${response.status}).`);
  const body = record(rawBody, "weather");
  return { current: readCurrent(body), hourly: readHourly(body), daily: readDaily(body) };
}

export async function getAirQualityData(latitude: number, longitude: number): Promise<AirQualityData | null> {
  const url = new URL("https://air-quality-api.open-meteo.com/v1/air-quality");
  url.searchParams.set("latitude", String(latitude));
  url.searchParams.set("longitude", String(longitude));
  url.searchParams.set("timezone", "auto");
  url.searchParams.set("forecast_days", "7");
  url.searchParams.set("hourly", "us_aqi,european_aqi,pm2_5,pm10");
  try {
    const { response, body: rawBody } = await requestJson(url.href);
    if (!response.ok) return null;
    const body = record(rawBody, "air quality");
    const hourly = record(body.hourly, "air quality hourly");
    return {
      hourly: {
        time: stringArray(hourly.time, "air quality time"),
        usAqi: numberArray(hourly.us_aqi, "US AQI"),
        europeanAqi: numberArray(hourly.european_aqi, "European AQI"),
        pm25: numberArray(hourly.pm2_5, "PM2.5"),
        pm10: numberArray(hourly.pm10, "PM10"),
      },
    };
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
  const { response, body: rawBody } = await requestJson(url.href);
  if (!response.ok) throw new Error(`Search unavailable (${response.status}).`);
  const body = record(rawBody, "location search");
  const results = body.results;
  if (results === undefined) return [];
  if (!Array.isArray(results)) throw new Error("Invalid location search results.");
  return results.flatMap((item): GeocodingResult[] => {
    if (typeof item !== "object" || item === null) return [];
    const result = item as JsonRecord;
    if (typeof result.id !== "number" || typeof result.name !== "string"
      || typeof result.latitude !== "number" || typeof result.longitude !== "number"
      || typeof result.country !== "string" || typeof result.country_code !== "string") return [];
    return [{
      id: result.id,
      name: result.name,
      latitude: result.latitude,
      longitude: result.longitude,
      country: result.country,
      countryCode: result.country_code,
      admin1: typeof result.admin1 === "string" ? result.admin1 : undefined,
    }];
  });
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
  const day = new Date(Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3]))).getUTCDay();
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
  if (code >= 61 && code <= 67 || code >= 80 && code <= 82) return isDay === 1 ? "rain" : "nightRain";
  if (code >= 71 && code <= 77 || code === 85 || code === 86) return isDay === 1 ? "snow" : "nightSnow";
  if (code >= 95) return isDay === 1 ? "storm" : "nightStorm";
  return "cloud";
}
