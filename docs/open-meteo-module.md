---
title: "Build an Open-Meteo module"
description: "Create a TypeScript-only Ink module for weather forecasts, place search, and air quality."
---

This guide builds `@example/ink-open-meteo`, a source module that uses `@ink/network`. It does not need a native adapter because Ink already provides the required HTTP and cache capabilities.

The examples follow the current [Open-Meteo forecast](https://open-meteo.com/en/docs), [geocoding](https://open-meteo.com/en/docs/geocoding-api), and [licence](https://open-meteo.com/en/license) documentation.

## Create the package

Create this source tree:

```text
ink-open-meteo/
├── package.json
└── src/
    ├── index.ts
    ├── client.ts
    ├── domain.ts
    ├── errors.ts
    ├── attribution.tsx
    └── internal/
        ├── forecast.ts
        ├── geocoding.ts
        └── normalise.ts
```

Add the Ink export and networking dependency:

```json
{
  "name": "@example/ink-open-meteo",
  "version": "1.0.0",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "ink": "./src/index.ts"
    }
  },
  "peerDependencies": {
    "ink": "*",
    "@ink/network": "*"
  },
  "peerDependenciesMeta": {
    "@ink/system": { "optional": true }
  }
}
```

The package publishes compiler-readable TypeScript. It does not bundle the Open-Meteo JavaScript SDK or run package code on the phone.

## Define the public model

Keep provider response fields out of app code. Add the public types to `src/domain.ts`:

```ts
export type ForecastInput = {
  latitude: number;
  longitude: number;
  units?: "metric" | "imperial";
  days?: 1 | 3 | 5 | 7 | 10 | 14 | 16;
  timezone?: "auto" | string;
};

export type Forecast = {
  location: {
    latitude: number;
    longitude: number;
    elevationMetres: number | null;
    timezone: string;
  };
  units: "metric" | "imperial";
  current: CurrentConditions;
  hourly: ReadonlyArray<HourlyConditions>;
  daily: ReadonlyArray<DailyConditions>;
};

export type CurrentConditions = {
  observedAtMs: number;
  temperature: number;
  apparentTemperature: number;
  precipitation: number;
  windSpeed: number;
  weather: WeatherCondition;
  isDay: boolean;
};

export type HourlyConditions = {
  observedAtMs: number;
  temperature: number;
  precipitationProbability: number | null;
  weather: WeatherCondition;
};

export type DailyConditions = {
  date: string;
  minimumTemperature: number;
  maximumTemperature: number;
  precipitationProbability: number | null;
  sunriseAtMs: number;
  sunsetAtMs: number;
  weather: WeatherCondition;
};

export type WeatherCondition =
  | "clear"
  | "partly-cloudy"
  | "overcast"
  | "fog"
  | "rain"
  | "snow"
  | "thunderstorm"
  | "unknown";
```

Use epoch milliseconds for instants. Keep daily calendar values as ISO `YYYY-MM-DD` strings in the forecast's resolved timezone.

## Define module errors

Add a small tagged union to `src/errors.ts`:

```ts
export type OpenMeteoError =
  | { kind: "invalid-input"; field: string; message: string; retryable: false }
  | { kind: "network"; message: string; retryable: true }
  | { kind: "rate-limited"; retryAfterMs: number | null; message: string; retryable: true }
  | { kind: "provider"; reason: string; message: string; retryable: false }
  | { kind: "invalid-response"; message: string; retryable: true };
```

Translate `@ink/network` errors inside the package. App screens should not depend on HTTP client errors or Open-Meteo's error response shape.

## Describe the provider response

Add private response types to `src/internal/forecast.ts`. Include only the fields requested by the package:

```ts
export type OpenMeteoForecastResponse = {
  latitude: number;
  longitude: number;
  elevation?: number;
  generationtime_ms: number;
  utc_offset_seconds: number;
  timezone: string;
  current: {
    time: string;
    temperature_2m: number;
    apparent_temperature: number;
    precipitation: number;
    weather_code: number;
    wind_speed_10m: number;
    is_day: 0 | 1;
  };
  hourly: {
    time: ReadonlyArray<string>;
    temperature_2m: ReadonlyArray<number>;
    precipitation_probability: ReadonlyArray<number | null>;
    weather_code: ReadonlyArray<number>;
  };
  daily: {
    time: ReadonlyArray<string>;
    temperature_2m_min: ReadonlyArray<number>;
    temperature_2m_max: ReadonlyArray<number>;
    precipitation_probability_max: ReadonlyArray<number | null>;
    sunrise: ReadonlyArray<string>;
    sunset: ReadonlyArray<string>;
    weather_code: ReadonlyArray<number>;
  };
};
```

Ink generates a response schema from this type. A missing required field or incompatible value fails the resource before normalisation runs.

## Normalise the forecast

Open-Meteo returns parallel arrays for hourly and daily values. Convert them into records in `src/internal/normalise.ts`:

```ts
export function normaliseForecast(
  response: OpenMeteoForecastResponse,
  units: "metric" | "imperial",
): Forecast {
  assertEqualLengths(response.daily);
  assertEqualLengths(response.hourly);

  return {
    location: {
      latitude: response.latitude,
      longitude: response.longitude,
      elevationMetres: response.elevation ?? null,
      timezone: response.timezone,
    },
    units,
    current: normaliseCurrent(response.current),
    hourly: zipHourly(response.hourly),
    daily: zipDaily(response.daily),
  };
}
```

These helpers use Ink's pure TypeScript subset. The compiler lowers them into schema-checked transformations in `app.ink`; Android does not execute this function as JavaScript.

Map unrecognised WMO weather codes to `unknown`. Reject unequal array lengths and invalid timestamps as `invalid-response` instead of returning partial records.

## Create the forecast resource

Add the provider call to `src/client.ts`:

```ts
import { cachedJson } from "@ink/network";
import type { ForecastInput } from "./domain";
import type { OpenMeteoForecastResponse } from "./internal/forecast";
import { normaliseForecast } from "./internal/normalise";
import { toOpenMeteoError } from "./errors";

const FORECAST_URL = "https://api.open-meteo.com/v1/forecast";

export function forecast(input: ForecastInput) {
  const units = input.units ?? "metric";
  const request = cachedJson<OpenMeteoForecastResponse>(FORECAST_URL, {
    query: {
      latitude: input.latitude,
      longitude: input.longitude,
      current: [
        "temperature_2m",
        "apparent_temperature",
        "precipitation",
        "weather_code",
        "wind_speed_10m",
        "is_day",
      ],
      hourly: [
        "temperature_2m",
        "precipitation_probability",
        "weather_code",
      ],
      daily: [
        "temperature_2m_min",
        "temperature_2m_max",
        "precipitation_probability_max",
        "sunrise",
        "sunset",
        "weather_code",
      ],
      forecast_days: input.days ?? 7,
      timezone: input.timezone ?? "auto",
      temperature_unit: units === "imperial" ? "fahrenheit" : "celsius",
      wind_speed_unit: units === "imperial" ? "mph" : "kmh",
      precipitation_unit: units === "imperial" ? "inch" : "mm",
    },
    maxAgeMs: 15 * 60_000,
    staleIfErrorMs: 6 * 60 * 60_000,
  });

  return request
    .map((response) => normaliseForecast(response, units))
    .mapError(toOpenMeteoError);
}
```

The module owns the requested variables, provider parameter names, units, cache duration, and error translation. A screen receives one `Resource<Forecast, OpenMeteoError>`.

`@ink/network` owns HTTPS, timeouts, cancellation, conditional requests, and safe GET retry policy. Do not reproduce those behaviours in this package.

## Export the client

Add the public object to `src/index.ts`:

```ts
import { forecast } from "./client";

export const OpenMeteo = {
  forecast,
};

export { OpenMeteoAttribution } from "./attribution";
export type {
  CurrentConditions,
  DailyConditions,
  Forecast,
  ForecastInput,
  HourlyConditions,
  WeatherCondition,
} from "./domain";
export type { OpenMeteoError } from "./errors";
```

Keep internal provider types unexported. This lets the provider response change without forcing every app to change.

## Use the module in an app

Install the local package in an example app, then render the resource:

```tsx
import { OpenMeteo, OpenMeteoAttribution } from "@example/ink-open-meteo";
import { Screen, Stack, Text, match } from "ink";

export default function ForecastScreen() {
  const forecast = OpenMeteo.forecast({
    latitude: 51.5072,
    longitude: -0.1276,
    units: "metric",
    days: 7,
  });

  return (
    <Screen title="Forecast">
      {match(forecast, {
        loading: () => <Text>Loading forecast</Text>,
        ready: ({ value }) => (
          <Stack gap={12}>
            <Text>{value.current.temperature}°</Text>
            <Text>{value.daily[0].maximumTemperature}° high</Text>
            <OpenMeteoAttribution />
          </Stack>
        ),
        stale: ({ value }) => (
          <Stack gap={12}>
            <Text>{value.current.temperature}°</Text>
            <Text>Forecast may be out of date</Text>
            <OpenMeteoAttribution />
          </Stack>
        ),
        error: ({ error }) => <Text>{error.message}</Text>,
      })}
    </Screen>
  );
}
```

Call `forecast.reload()` from a refresh button when the user wants a new result. The resource does not poll while the screen is inactive.

## Add place search

Use the geocoding endpoint for named places:

```ts
const places = OpenMeteo.search({
  query: query.value,
  language: "en",
  limit: 8,
});
```

Start requests after the query contains at least two Unicode characters. Debounce changing input for 300 milliseconds and cancel the older request when the query changes.

Return a `Place` with a provider ID, display name, country code, administrative area, coordinates, elevation, and timezone. Convert missing optional fields to `null` so screens receive one stable shape.

Cache search results for seven days and allow a 30-day stale fallback. Include normalised query, language, country filter, limit, endpoint, and schema version in the cache key.

## Add air quality

Expose air quality as a separate resource:

```ts
const air = OpenMeteo.airQuality({
  latitude,
  longitude,
  index: "european",
  includePollen: true,
});
```

Return current index, category, particulate values, UV index, and an hourly series. Use `null` for pollen fields that are unavailable outside a supported region or season.

Keep air-quality cache and attribution metadata separate from the weather forecast because the endpoint and upstream data providers differ.

## Add attribution

Open-Meteo data require attribution. Add a component that opens the provider page through `@ink/system`:

```tsx
import { systemAction } from "@ink/system";
import { Button } from "ink";

export function OpenMeteoAttribution() {
  const attribution = systemAction({
    kind: "web",
    url: "https://open-meteo.com/",
  });

  return (
    <Button onPress={() => attribution.run()}>
      Weather data by Open-Meteo.com
    </Button>
  );
}
```

Render the attribution with fresh, cached, and stale data. If the module materially changes source data, describe that change next to the attribution.

## Support customer and self-hosted endpoints

Add a client factory for apps that cannot use the public endpoint:

```ts
import { secret } from "@ink/secure-store";

const apiKey = secret("open-meteo.api-key");

if (apiKey.status === "ready") {
  const WeatherProvider = createOpenMeteo({
    endpoint: "https://customer-api.open-meteo.com",
    apiKey: apiKey.secret,
  });

  const forecast = WeatherProvider.forecast({ latitude, longitude });
}
```

Allow the official public endpoint, official customer endpoint, or an application allow-listed HTTPS self-hosted endpoint. Mark `apikey` as a sensitive query field so `@ink/network` redacts it from logs and diagnostics.

A key in a client APK can be extracted. Use a restricted key or an application service when the provider's policy requires stronger protection.

The public endpoint is for non-commercial use and applies current usage limits. Link to the [Open-Meteo terms](https://open-meteo.com/en/terms) instead of copying limits into package logic.

## Check the package

Run the compiler and inspect the linked app:

```sh
ink package build
ink check
ink info
ink preview
```

The package should add network and cache capabilities only. If `ink info` shows a native module, Android SDK, or new permission, the implementation has crossed a boundary that this provider does not need.
