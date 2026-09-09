import { location } from "@ink/location";
import { NativeError } from "ink/native";
import { useMemo } from "react";
import { resource, useSnapshot, type SnapshotSource } from "ink";
import { weatherPlace, type Place } from "../lib/place";
import type { SavedLocation } from "../lib/preferences";

const LOCATION_STALE_TIME = 900_000;
const currentLocation = resource({
  key: () => [],
  load: async (): Promise<Place> => {
    let permission = await location.getPermission("balanced");
    if (permission !== "granted") permission = await location.requestPermission("balanced");
    if (permission !== "granted") {
      throw new Error(permission === "blocked"
        ? "Location access is blocked. Enable it in the phone’s settings, then try again. You can also choose a saved location in Locations or Settings."
        : "Location access is needed for local weather. Try again to request it, or choose a saved location in Locations or Settings.");
    }
    try {
      // Approximate location updates can be throttled to ten-minute intervals.
      const fix = await location.current({ accuracy: "balanced", maximumAge: LOCATION_STALE_TIME, timeout: 15_000 });
      return { key: `current:${fix.latitude}:${fix.longitude}`, label: "Current Location", latitude: fix.latitude, longitude: fix.longitude };
    } catch (cause) {
      if (cause instanceof NativeError && cause.kind === "timeout") {
        throw new Error("Could not find your location. Try again, or choose a saved location in Locations.");
      }
      throw cause;
    }
  },
  staleTime: LOCATION_STALE_TIME,
  refreshInterval: LOCATION_STALE_TIME,
});
const emptyState = { status: "ready", data: null } as const;
const empty: SnapshotSource<null> = { getSnapshot: () => emptyState, subscribe: () => () => {} };

export function useCurrentPlace(mainLocation: SavedLocation | null) {
  const selectedMainPlace = useMemo(() => mainLocation ? weatherPlace(mainLocation) : null, [mainLocation]);
  const source = selectedMainPlace ? null : currentLocation();
  const state = useSnapshot<Place | null>(source ?? empty);
  return {
    place: selectedMainPlace ?? (state.status === "ready" ? state.data : null),
    error: state.status === "error" ? state.error.message : null,
    loading: state.status === "loading",
    retry: () => { void source?.refresh(); },
  };
}
