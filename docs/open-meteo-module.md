---
title: "Build an Open-Meteo module"
description: "Compile a typed weather provider into a small Ink source module."
tag: "Planned"
---

This guide builds a source module that exposes one domain resource: `OpenMeteo.forecast()`. The package owns provider query details, response validation, normalisation, caching, errors, attribution, and preview fixtures.

The example deliberately starts with one operation. Add search or air quality only after the forecast interface compiles and works end to end.

## Create the package

```text
ink-open-meteo/
├── package.json
├── preview/
│   ├── forecast.json
│   └── scenarios.ts
└── src/
    ├── domain.ts
    ├── index.tsx
    └── provider.ts
```

```json
{
  "name": "@example/ink-open-meteo",
  "version": "1.0.0",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "ink": "./src/index.tsx"
    }
  },
  "ink": {
    "preview": "./dist/preview.json"
  },
  "peerDependencies": {
    "ink": "*",
    "@ink/network": "*",
    "@ink/system": "*"
  }
}
```

Open-Meteo is a true external dependency. The HTTPS provider is the production adapter and the deterministic fixture is the preview adapter.

## Define the public model

Keep provider field names out of the app-facing interface:

```ts
export type ForecastRequest = {
  latitude: number;
  longitude: number;
  days?: number;
  timezone?: string;
  units?: "metric" | "imperial";
};

export type Forecast = {
  current: {
    temperature: number;
    apparentTemperature: number;
    weatherCode: number;
    observedAt: string;
  };
  daily: ReadonlyArray<{
    date: string;
    minimumTemperature: number;
    maximumTemperature: number;
    weatherCode: number;
  }>;
  temperatureUnit: "celsius" | "fahrenheit";
  timezone: string;
};
```

`days` accepts 1 to 16 and defaults to 7. `timezone` is an IANA timezone string and defaults to `"auto"` internally. Do not type it as `"auto" | string`, which collapses to `string` and gives callers no additional information.

## Define module errors

Expose errors an app can act on:

```ts
export type OpenMeteoError =
  | {
      kind: "invalid-request";
      message: string;
      field: "latitude" | "longitude" | "days" | "timezone";
      retryable: false;
    }
  | {
      kind: "offline";
      message: string;
      retryable: true;
    }
  | {
      kind: "rate-limited";
      message: string;
      retryAtMs: number | null;
      retryable: true;
    }
  | {
      kind: "provider" | "invalid-response";
      message: string;
      retryable: boolean;
    };
```

Use absolute `retryAtMs` consistently with Network. Do not expose provider payloads or HTTP client details.

## Describe the provider response

Declare only fields the implementation consumes. Optional provider diagnostics should not make an otherwise valid forecast fail validation.

```ts
type OpenMeteoForecastResponse = {
  timezone: string;
  current_units: {
    temperature_2m: "°C" | "°F";
  };
  current: {
    time: string;
    temperature_2m: number;
    apparent_temperature: number;
    weather_code: number;
  };
  daily: {
    time: ReadonlyArray<string>;
    temperature_2m_min: ReadonlyArray<number>;
    temperature_2m_max: ReadonlyArray<number>;
    weather_code: ReadonlyArray<number>;
  };
};
```

Ink validates this type before normalisation runs.

## Normalise the response

Use pure source-module functions:

```ts
import { parseIsoDateTime, zipExact } from "ink/module";

function normaliseForecast(response: OpenMeteoForecastResponse): Forecast {
  const daily = zipExact({
    date: response.daily.time,
    minimumTemperature: response.daily.temperature_2m_min,
    maximumTemperature: response.daily.temperature_2m_max,
    weatherCode: response.daily.weather_code,
  });

  return {
    current: {
      temperature: response.current.temperature_2m,
      apparentTemperature: response.current.apparent_temperature,
      weatherCode: response.current.weather_code,
      observedAt: parseIsoDateTime(response.current.time),
    },
    daily,
    temperatureUnit: response.current_units.temperature_2m === "°F"
      ? "fahrenheit"
      : "celsius",
    timezone: response.timezone,
  };
}
```

`zipExact()` is an approved pure intrinsic that fails when provider arrays have different lengths. `parseIsoDateTime()` validates and normalises an ISO timestamp without reading the current clock.

## Define the forecast resource

Use `defineResource()` to declare the app-facing input and domain error:

```ts
import {
  defineResource,
  ianaTimezone,
  integerBetween,
  literal,
  numberBetween,
  optional,
  taggedErrors,
} from "ink/module";
import { json } from "@ink/network";

const FORECAST_URL = "https://api.open-meteo.com/v1/forecast";

export const forecast = defineResource<Forecast>({
  input: {
    latitude: numberBetween(-90, 90),
    longitude: numberBetween(-180, 180),
    days: optional(integerBetween(1, 16)),
    timezone: optional(ianaTimezone()),
    units: optional(literal("metric", "imperial")),
  },
  errors: taggedErrors<OpenMeteoError>(),
  load: (input) => json<OpenMeteoForecastResponse>(FORECAST_URL, {
    query: {
      latitude: input.latitude,
      longitude: input.longitude,
      forecast_days: input.days ?? 7,
      timezone: input.timezone ?? "auto",
      temperature_unit: input.units === "imperial" ? "fahrenheit" : "celsius",
      current: [
        "temperature_2m",
        "apparent_temperature",
        "weather_code",
      ],
      daily: [
        "temperature_2m_min",
        "temperature_2m_max",
        "weather_code",
      ],
    },
    queryEncoding: "repeat",
    cache: {
      freshForMs: 300_000,
      staleIfErrorForMs: 86_400_000,
    },
  }).map(normaliseForecast).mapError(toOpenMeteoError),
});
```

`defineResource()` is package-author syntax. Apps still call a normal typed function. The compiler specialises its pure transformation and Network operation into the source-module IR; no callback or JavaScript runtime is shipped.

The package owns provider-specific query-list encoding, cache policy, validation, and error translation. Apps can call `reload()` on the returned resource without learning those details.

## Export the domain interface

```tsx
import { systemAction } from "@ink/system";
import { Button } from "ink";
import { forecast } from "./provider";

export const OpenMeteo = { forecast };

export function OpenMeteoAttribution() {
  const website = systemAction({
    kind: "web",
    url: "https://open-meteo.com/",
  });

  return (
    <Button onPress={() => website.run()}>
      Weather data by Open-Meteo.com
    </Button>
  );
}
```

The explicit `@ink/system` peer dependency matches this import. Attribution remains part of the module interface rather than copied into each app.

## Add preview scenarios

Record a small provider fixture containing exactly the fields used by the response type. Then declare deterministic scenarios:

```ts
export default defineSourcePreview({
  operations: {
    forecast: {
      scenarios: {
        clear: responseFixture("./forecast.json"),
        offline: networkError("offline"),
        rateLimited: networkError("rate-limited", {
          retryAtMs: 1_800_000_000_000,
        }),
        malformed: responseFixture("./forecast-malformed.json"),
      },
    },
  },
});
```

Preview fixtures are validated against the provider response type during `ink package build`. They do not contact Open-Meteo and are not included in the APK.

## Use the module

```tsx
const forecast = OpenMeteo.forecast({
  latitude: 51.5072,
  longitude: -0.1276,
  days: 7,
  units: "metric",
});

{match(forecast, {
  loading: () => <Text>Loading forecast</Text>,
  ready: ({ value, freshness, warning }) => (
    <Stack gap={12}>
      <Text>{value.current.temperature}°</Text>
      {freshness === "stale" ? <Text>Showing saved weather</Text> : null}
      {warning ? <Button onPress={() => forecast.reload()}>Retry</Button> : null}
      <OpenMeteoAttribution />
    </Stack>
  ),
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

This app-facing interface stays small while the module implementation owns the true external provider seam.

## Check the package

```sh
ink package build
ink check
ink preview
ink info
```

The acceptance criterion is stronger than type-checking: install the packed module into an example app and compile every documented snippet through the real source-module pipeline. `ink info` should show only Network and System web actions, with no native module or JavaScript runtime.
