import { formatLocationName, type SavedLocation } from "./preferences";

export type Place = {
  key: string;
  label: string;
  latitude: number;
  longitude: number;
};

export function weatherPlace(saved: SavedLocation): Place {
  return {
    key: `${saved.id}:${saved.latitude}:${saved.longitude}`,
    label: formatLocationName(saved),
    latitude: saved.latitude,
    longitude: saved.longitude,
  };
}
