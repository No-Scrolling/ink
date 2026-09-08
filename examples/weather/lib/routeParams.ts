import type { SavedLocation } from "./preferences";

export function routeQuery(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function routeNumber(value: unknown): number {
  if (typeof value === "number") return value;
  return typeof value === "string" && value.trim() !== "" ? Number(value) : NaN;
}

export function decodeWeatherRoute(value: unknown): { location: SavedLocation | null } {
  if (typeof value !== "object" || value === null) return { location: null };
  const id = routeNumber("id" in value ? value.id : undefined);
  const latitude = routeNumber("latitude" in value ? value.latitude : undefined);
  const longitude = routeNumber("longitude" in value ? value.longitude : undefined);
  if (!Number.isSafeInteger(id) || id < 0 || !Number.isFinite(latitude) || latitude < -90 || latitude > 90
    || !Number.isFinite(longitude) || longitude < -180 || longitude > 180) return { location: null };
  return { location: {
    id, latitude, longitude,
    name: routeQuery("name" in value ? value.name : undefined) || "Selected Location",
    country: routeQuery("country" in value ? value.country : undefined),
    admin1: routeQuery("admin1" in value ? value.admin1 : undefined) || undefined,
  } };
}
