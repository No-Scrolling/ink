---
title: "Build a weather module"
description: "A normal TypeScript provider function, a decoder and an Ink screen."
tag: "Design specification"
---

Start with a local `weather.ts`. It can become an npm package later without changing its programming model. This walkthrough uses Open-Meteo's JSON endpoint; the provider also publishes a client that you can assess against Ink's [host profile](runtime.md).

Enable networking:

```toml
[android]
capabilities = ["network"]
```

## Fetch and decode

```ts
// weather.ts
export type Place = { latitude: number; longitude: number };
export type Forecast = { temperature: number; observedAt: number };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function decodeForecast(value: unknown): Forecast {
  if (!isRecord(value) || !isRecord(value.current)) {
    throw new Error("Missing current weather");
  }
  const { temperature_2m: temperature, time } = value.current;
  if (typeof temperature !== "number" || !Number.isFinite(temperature)
    || typeof time !== "number" || !Number.isFinite(time)) {
    throw new Error("Invalid current weather");
  }
  return { temperature, observedAt: time * 1000 };
}

export async function getForecast(place: Place, options: {
  signal?: AbortSignal;
} = {}): Promise<Forecast> {
  const url = new URL("https://api.open-meteo.com/v1/forecast");
  url.searchParams.set("latitude", String(place.latitude));
  url.searchParams.set("longitude", String(place.longitude));
  url.searchParams.set("current", "temperature_2m");
  url.searchParams.set("temperature_unit", "celsius");
  url.searchParams.set("timeformat", "unixtime");
  const signal = options.signal
    ? AbortSignal.any([options.signal, AbortSignal.timeout(15_000)])
    : AbortSignal.timeout(15_000);
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`Weather unavailable (${response.status})`);
  const body: unknown = await response.json();
  return decodeForecast(body);
}
```

The request selects current temperature in Celsius and Unix timestamps, as described by the [Open-Meteo API](https://open-meteo.com/en/docs). The decoder converts seconds to milliseconds and keeps provider fields out of the UI. Validate user-entered coordinates before calling it. A schema library can replace the manual decoder without changing the function's callers.

## Show it

```tsx
// App.tsx
import { Button, Screen, Text, useResource } from "ink";
import { getForecast } from "./weather";

const london = { latitude: 51.5074, longitude: -0.1278 };

export default function Weather() {
  const forecast = useResource(
    ["forecast", london.latitude, london.longitude, "celsius"],
    ({ signal }) => getForecast(london, { signal }),
    { staleTime: 300_000 },
  );
  return (
    <Screen title="London">
      {forecast.status === "loading" && <Text>Loading forecast</Text>}
      {forecast.status === "error" && <Text>Could not load the forecast</Text>}
      {forecast.status === "ready" && <>
        <Text size={64}>{Math.round(forecast.data.temperature)}°</Text>
        <Text>Updated {new Date(forecast.data.observedAt).toISOString()}</Text>
        {forecast.warning && <Text>Could not refresh · showing the previous forecast</Text>}
      </>}
      <Button onPress={forecast.reload} disabled={forecast.refreshing}>Refresh</Button>
    </Screen>
  );
}
```

The resource observes while visible, deduplicates the keyed read and retains the last successful value during a failed refresh. It is an in-memory cache. This minimal screen does not yet provide offline availability after a restart.

## Grow the app

Persist saved places and units in [Store](store.md). Cache decoded forecasts in [Records](records.md), including coordinates, requested units, provider timestamp and fetch timestamp. A domain function can return saved data immediately and publish a later refresh through a readable source. Show its age and distinguish a stale value from a failed initial load.

Place search is a separate provider function. Current location is an optional [Location](location.md) action; a saved place works without permission. Hourly and daily views need bounded requested fields and correct handling of the forecast location's time zone.

Use a [background task](background.md) only if fresh saved forecasts materially help the app. The job calls the same function and persists results; it does not mount this screen or share its resource cache.

## Package it

Export `getForecast`, its types and any provider-specific error classes through normal package exports. Keep a React hook in an optional UI entry point. Choose an injected fetch-compatible transport only if consumers need that seam; no special compiler plugin is required.

A provider client from npm can sit behind this same interface if its transport and binary-decoding requirements work in Ink. Check licensing, attribution and service terms for the app's use through the [provider documentation](https://open-meteo.com/en/docs).
