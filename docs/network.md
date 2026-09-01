---
title: "Network"
description: "Read and update typed JSON over HTTPS."
tag: "Partial"
---

`@ink/network` provides typed JSON resources and explicit mutations. Ink validates every response against its TypeScript type before it reaches your app.

## Read JSON

Use `json<T>()` for an HTTPS GET request. The resource starts when its screen becomes active.

```tsx
import { json } from "@ink/network";
import { Screen, Text, match } from "ink";

type Forecast = { temperature: number; summary: string };

const forecast = json<Forecast>("https://weather.example/forecast", {
  query: { city: "London" },
  timeoutMs: 15_000,
});

<Screen title="Forecast">
  {match(forecast, {
    loading: () => <Text>Loading forecast</Text>,
    ready: ({ value }) => <Text>{value.temperature}</Text>,
    error: ({ error }) => <Text>{error.message}</Text>,
  })}
</Screen>
```

Query values can be strings, numbers, booleans, or lists of those values. A provider module owns any provider-specific list encoding. URLs must use HTTPS, except for emulator loopback addresses.

`timeoutMs` accepts 1,000 to 120,000 milliseconds and defaults to 15 seconds. Responses are limited to 1 MiB after decompression.

## Cache a read

Pass `cache` when a response can be reused across app launches:

```tsx
const forecast = json<Forecast>("https://weather.example/forecast", {
  cache: {
    freshForMs: 300_000,
    staleIfErrorForMs: 86_400_000,
  },
});
```

`json()` always uses `loading`, `ready`, or `error`. A cached ready result adds:

- `freshness` as `"fresh"` or `"stale"`;
- `activity` as `"idle"` or `"refreshing"`;
- `updatedAtMs`;
- `warning` when an older value remains usable after a refresh error.

This lets ordinary screens render one ready branch while screens that care about freshness can show it. Changing the response type invalidates an incompatible cached value. Call `reload()` to bypass freshness and replace the current request.

## Transform a response

Use `map()` inside an app or source module to turn a validated provider response into a domain model. Use `mapError()` to expose a smaller tagged error contract.

```tsx
const forecast = json<ProviderForecast>(url, {
  cache: { freshForMs: 300_000 },
}).map((response) => ({
  temperature: response.current.temperature_2m,
  summary: weatherSummary(response.current.weather_code),
})).mapError(toWeatherError);
```

Mapping uses Ink's closed pure-expression subset. It can construct values, map and filter lists, zip validated lists, narrow tagged values, and call other pure source-module functions. It cannot perform I/O, mutate state, read the clock, or call a native operation.

Mapping preserves caching, cancellation, and reload behaviour. A failed transformation returns `invalid-response` unless `mapError()` translates it.

## Send a mutation

Use `mutation<T>()` for a `POST`, `PUT`, `PATCH`, or `DELETE` request. It remains `idle` until you call `run()`.

```tsx
import { mutation } from "@ink/network";
import { Button, Text, match, state } from "ink";

const placeName = state("London");
const save = mutation<{ id: string }>("https://weather.example/places", {
  method: "POST",
  body: { name: placeName.value },
});

{match(save, {
  idle: () => <Button onPress={() => save.run()}>Save place</Button>,
  running: () => <Text>Saving place</Text>,
  success: ({ value }) => <Text>Saved {value.id}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

Ink materialises referenced state when `run()` starts. Calling `run()` while the mutation is running has no effect. Mutations are not retried automatically because repeating a write may not be safe.

## Authenticate a request

Pass a stable opaque provider from Auth or Secure store. The resource remains declared while the provider restores, refreshes, or changes generation:

```tsx
const account = oauthSession(options);

const profile = json<Profile>("https://api.example/profile", {
  authorization: account.authorization,
});
```

Network waits while an authorisation is restoring. It reloads after the reference changes and returns `authentication-required` when no usable credential exists.

For a manually supplied bearer token, pass a Secure store slot to `bearer()`:

```tsx
const apiKey = secret("weather.api-key");

const response = json<Result>(url, {
  query: { apikey: sensitive(apiKey) },
});
```

Sensitive values carry their redaction policy with them. Ink redacts them from logs, errors, cache metadata, and `ink info`. They remain visible to the remote server and can still be extracted from a distributed client app.

## Create a background request plan

Use `getJson<T>()` as static work inside a Background task. The task owns its latest durable result:

```tsx
const refresh = backgroundTask({
  key: "forecast.refresh",
  schedule: { kind: "periodic", everyMinutes: 30 },
  work: getJson<Forecast>("https://weather.example/forecast"),
});
```

`getJson()` describes work and does not start a request where it is declared. Plans can use opaque background-safe authorisation but cannot capture screen state or temporary handles.

The compiler rejects direct `Authorization`, `Cookie`, and proxy credential headers. Authorisation is removed from cross-origin redirects.

## Lifecycle and errors

Reads are screen-scoped. Leaving the screen or calling `reload()` cancels observation and ignores late results. Terminating the underlying request is best effort. Identical active GET requests share one in-flight operation.

GET requests retry one transient connection or server failure within the original timeout. Redirects are limited to five and cannot downgrade from HTTPS to HTTP.

Errors distinguish offline access, timeout, authentication, forbidden requests, missing resources, rate limits, server failures, invalid responses, oversized responses, unavailable networking, and unexpected failures. Every error provides `kind`, `message`, and `retryable`. Rate-limit errors provide `retryAtMs` when the server supplies a valid retry time.
