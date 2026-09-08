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
export const DETAIL_LABELS: Record<WeatherDetail, string> = {
  Temp: "Temperature", "Feels Like": "Feels like", "Precip Chance": "Precip. chance",
  "Precip Amount": "Precip. amount", "Wind Speed": "Wind speed", "Wind Gusts": "Wind gusts",
  "UV Index": "UV index", Humidity: "Humidity", "Dew Point": "Dew point", "Cloud Cover": "Cloud cover",
  Visibility: "Visibility", Pressure: "Pressure", "AQI (US)": "AQI (US)",
  "AQI (EU)": "AQI (EU)", "PM2.5": "PM2.5", PM10: "PM10",
};

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

function isSavedLocation(value: unknown): value is SavedLocation {
  if (typeof value !== "object" || value === null) return false;
  const location = value as Record<string, unknown>;
  return typeof location.id === "number"
    && Number.isSafeInteger(location.id)
    && typeof location.name === "string"
    && (location.admin1 === undefined || typeof location.admin1 === "string")
    && typeof location.country === "string"
    && typeof location.latitude === "number"
    && Number.isFinite(location.latitude)
    && Math.abs(location.latitude) <= 90
    && typeof location.longitude === "number"
    && Number.isFinite(location.longitude)
    && Math.abs(location.longitude) <= 180;
}

function decodeDetails(value: unknown): WeatherDetail[] {
  if (!Array.isArray(value)) throw new Error("Invalid weather details");
  const details = value.filter((detail): detail is WeatherDetail =>
    typeof detail === "string" && (WEATHER_DETAILS as readonly string[]).includes(detail),
  );
  const unique = [...new Set(details)];
  if (unique.length === 0) return [...initial.selectedDetails];
  return unique;
}

export const preferences = createStore<WeatherPreferences>({
  key: "weather.preferences",
  version: 1,
  initial,
  decode(value) {
    if (typeof value !== "object" || value === null) {
      throw new Error("Could not read weather settings.");
    }
    const saved = value as Record<string, unknown>;
    const temperatureUnit = saved.temperatureUnit;
    const windSpeedUnit = saved.windSpeedUnit;
    const precipitationUnit = saved.precipitationUnit;
    const timeFormat = saved.timeFormat;
    const mainLocation = saved.mainLocation;
    const savedLocations = saved.savedLocations;
    if ((temperatureUnit !== "Celsius" && temperatureUnit !== "Fahrenheit")
      || (windSpeedUnit !== "km/h" && windSpeedUnit !== "m/s" && windSpeedUnit !== "mph" && windSpeedUnit !== "Knots")
      || (precipitationUnit !== "Millimeter" && precipitationUnit !== "Inch")
      || (timeFormat !== "24h" && timeFormat !== "12h")
      || (mainLocation !== null && !isSavedLocation(mainLocation))
      || !Array.isArray(savedLocations)
      || savedLocations.some(location => !isSavedLocation(location))) {
      throw new Error("Could not read weather settings.");
    }
    return {
      invertColours: saved.invertColours === true,
      temperatureUnit,
      windSpeedUnit,
      precipitationUnit,
      timeFormat,
      selectedDetails: decodeDetails(saved.selectedDetails),
      mainLocation,
      savedLocations,
    };
  },
});

export function formatLocationName(location: Pick<SavedLocation, "name" | "admin1" | "country">): string {
  const admin = location.admin1 && location.admin1 !== location.name ? `, ${location.admin1}` : "";
  return `${location.name}${admin}, ${location.country}`;
}
