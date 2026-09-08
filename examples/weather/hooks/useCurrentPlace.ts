import { location } from "@ink/location";
import { NativeError } from "ink/native";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { weatherPlace, type Place } from "../lib/place";
import type { SavedLocation } from "../lib/preferences";

export function useCurrentPlace(mainLocation: SavedLocation | null) {
  const mainKey = mainLocation
    ? `${mainLocation.id}:${mainLocation.latitude}:${mainLocation.longitude}`
    : "current";
  const selectedMainPlace = useMemo(
    () => mainLocation ? weatherPlace(mainLocation) : null,
    [mainKey],
  );
  const [place, setPlace] = useState<Place | null>(selectedMainPlace);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(!selectedMainPlace);
  const requestToken = useRef(0);
  const locationRequest = useRef<AbortController | null>(null);

  const requestCurrent = useCallback(async () => {
    const token = ++requestToken.current;
    locationRequest.current?.abort();
    const controller = new AbortController();
    locationRequest.current = controller;
    setLoading(true);
    setError(null);
    try {
      let permission = await location.getPermission("balanced");
      if (permission !== "granted") permission = await location.requestPermission("balanced");
      if (permission !== "granted") {
        if (token !== requestToken.current) return;
        setPlace(null);
        setError(permission === "blocked"
          ? "Location access is blocked. Enable it in the phone’s settings, then try again. You can also choose a saved location in Locations or Settings."
          : "Location access is needed for local weather. Try again to request it, or choose a saved location in Locations or Settings.");
        return;
      }
      if (token !== requestToken.current) return;
      // Approximate location updates can be throttled to ten-minute intervals.
      const fix = await location.current({ accuracy: "balanced", maximumAge: 900_000, timeout: 15_000, signal: controller.signal });
      if (token !== requestToken.current) return;
      setPlace({ key: `current:${fix.latitude}:${fix.longitude}`, label: "Current Location", latitude: fix.latitude, longitude: fix.longitude });
    } catch (cause) {
      if (token !== requestToken.current) return;
      setError(cause instanceof NativeError && cause.kind === "timeout"
        ? "Could not find your location. Try again, or choose a saved location in Locations."
        : cause instanceof Error ? cause.message : "Could not read your location.");
    } finally {
      if (token === requestToken.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (selectedMainPlace) {
      requestToken.current += 1;
      setPlace(selectedMainPlace);
      setError(null);
      setLoading(false);
    } else {
      setPlace(current => current?.key.startsWith("current:") ? current : null);
      void requestCurrent();
    }
    return () => {
      requestToken.current += 1;
      locationRequest.current?.abort();
    };
  }, [mainKey, requestCurrent, selectedMainPlace]);

  return { place, error, loading, retry: requestCurrent };
}
