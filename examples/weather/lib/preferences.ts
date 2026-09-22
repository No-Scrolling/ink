import * as v from "valibot";
import { createStore } from "@ink/store";

export const WEATHER_DETAILS = [
  "Temp",
  "Feels Like",
  "Precip Chance",
  "Precip Amount",
  "Wind Speed",
  "Wind Gusts",
  "UV Index",
  "Humidity",
  "Dew Point",
  "Cloud Cover",
  "Visibility",
  "Pressure",
  "AQI (US)",
  "AQI (EU)",
  "PM2.5",
  "PM10",
] as const;

export type WeatherDetail = (typeof WEATHER_DETAILS)[number];
export const DETAIL_LABELS = {
  Temp: "Temperature", "Feels Like": "Feels like", "Precip Chance": "Precip. chance",
  "Precip Amount": "Precip. amount", "Wind Speed": "Wind speed", "Wind Gusts": "Wind gusts",
  "UV Index": "UV index", Humidity: "Humidity", "Dew Point": "Dew point", "Cloud Cover": "Cloud cover",
  Visibility: "Visibility", Pressure: "Pressure", "AQI (US)": "AQI (US)",
  "AQI (EU)": "AQI (EU)", "PM2.5": "PM2.5", PM10: "PM10",
} satisfies Record<WeatherDetail, string>;

export type TemperatureUnit = "Celsius" | "Fahrenheit";
export type WindSpeedUnit = "km/h" | "m/s" | "mph" | "Knots";
export type PrecipitationUnit = "Millimeter" | "Inch";
export type TimeFormat = "24h" | "12h";

export interface SavedLocation {
  id: number;
  name: string;
  admin1?: string;
  country: string;
  latitude: number;
  longitude: number;
}

export interface WeatherPreferences {
  invertColours: boolean;
  temperatureUnit: TemperatureUnit;
  windSpeedUnit: WindSpeedUnit;
  precipitationUnit: PrecipitationUnit;
  timeFormat: TimeFormat;
  selectedDetails: WeatherDetail[];
  mainLocation: SavedLocation | null;
  savedLocations: SavedLocation[];
}

const initial: WeatherPreferences = {
  invertColours: false,
  temperatureUnit: "Celsius",
  windSpeedUnit: "km/h",
  precipitationUnit: "Millimeter",
  timeFormat: "24h",
  selectedDetails: ["Temp", "Feels Like", "Precip Chance"],
  mainLocation: null,
  savedLocations: [],
};

export const locationSchema = v.object({
  id: v.pipe(v.number(), v.safeInteger()),
  name: v.string(),
  admin1: v.optional(v.string()),
  country: v.string(),
  latitude: v.pipe(v.number(), v.finite(), v.minValue(-90), v.maxValue(90)),
  longitude: v.pipe(v.number(), v.finite(), v.minValue(-180), v.maxValue(180)),
});

const details = v.pipe(
  v.array(v.fallback(v.nullable(v.picklist(WEATHER_DETAILS)), null)),
  v.transform(values => {
    const selected = [...new Set(values.filter(value => value !== null))];
    return selected.length ? selected : [...initial.selectedDetails];
  }),
);

export const preferences = createStore<WeatherPreferences>({
  key: "weather.preferences",
  version: 1,
  initial,
  decode: v.parser(v.object({
    invertColours: v.fallback(v.boolean(), false),
    temperatureUnit: v.picklist(["Celsius", "Fahrenheit"]),
    windSpeedUnit: v.picklist(["km/h", "m/s", "mph", "Knots"]),
    precipitationUnit: v.picklist(["Millimeter", "Inch"]),
    timeFormat: v.picklist(["24h", "12h"]),
    selectedDetails: details,
    mainLocation: v.nullable(locationSchema),
    savedLocations: v.array(locationSchema),
  })),
});

export function formatLocationName(location: Pick<SavedLocation, "name" | "admin1" | "country">): string {
  const admin = location.admin1 && location.admin1 !== location.name ? `, ${location.admin1}` : "";
  return `${location.name}${admin}, ${location.country}`;
}
