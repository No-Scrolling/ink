import { lightos } from "@ink/lightos";
import { location } from "@ink/location";
import { NativeError, resource, useSnapshot } from "ink";
import type { Place } from "../lib/place";

const LOCATION_STALE_TIME = 900_000;
const currentLocation = resource({
  key: () => [],
  load: async (): Promise<Place> => {
    let permission = await lightos.getPermission("location-approximate");
    if (permission !== "granted")
      permission = await lightos.requestPermission("location-approximate");
    if (permission !== "granted") {
      throw new Error(
        permission === "blocked"
          ? "Location access is blocked. Enable it in the phone’s settings, then try again. You can also choose a saved location in Locations or Settings."
          : "Location access is needed for local weather. Try again to request it, or choose a saved location in Locations or Settings.",
      );
    }
    try {
      // Approximate location updates can be throttled to ten-minute intervals.
      const fix = await location.current({
        accuracy: "balanced",
        maximumAge: LOCATION_STALE_TIME,
        timeout: 15_000,
      });
      return {
        key: `current:${fix.latitude}:${fix.longitude}`,
        label: "Current Location",
        latitude: fix.latitude,
        longitude: fix.longitude,
      };
    } catch (cause) {
      if (cause instanceof NativeError && cause.kind === "timeout") {
        throw new Error(
          "Could not find your location. Try again, or choose a saved location in Locations.",
        );
      }
      throw cause;
    }
  },
  staleTime: LOCATION_STALE_TIME,
  refreshInterval: LOCATION_STALE_TIME,
});

export function useCurrentPlace() {
  const source = currentLocation();
  const state = useSnapshot(source);
  return {
    place: state.status === "ready" ? state.data : null,
    error:
      state.status === "error"
        ? state.error.message
        : state.status === "ready"
          ? (state.refreshError?.message ?? null)
          : null,
    loading: state.status === "loading",
    retry: source.refresh,
  };
}
