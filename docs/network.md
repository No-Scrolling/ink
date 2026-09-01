---
title: "Network"
description: "Read and update typed JSON over HTTPS."
tag: "Partial"
---

`@ink/network` provides typed JSON reads, durable response caching, explicit mutations, and authenticated requests. Ink validates every response against its TypeScript type before it reaches your app.

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

Query values can be strings, numbers, booleans, or lists of those values. Lists are encoded as comma-separated values. Header values can be literal strings or string state values. URLs must use HTTPS, except for emulator loopback addresses.

`timeoutMs` accepts 1,000 to 120,000 milliseconds and defaults to 15 seconds. Responses are limited to 1 MiB after decompression.

## Cache a read

Use `cachedJson<T>()` when a response can be reused across app launches.

```tsx
import { cachedJson } from "@ink/network";

const forecast = cachedJson<Forecast>("https://weather.example/forecast", {
  maxAgeMs: 300_000,
  staleIfErrorMs: 86_400_000,
});
```

| Option | Meaning | Default |
| --- | --- | --- |
| `maxAgeMs` | Return the saved value without a request while it remains fresh. | 5 minutes |
| `staleIfErrorMs` | Keep an older value available when a refresh fails. | 1 day |

A cached resource is `ready` while its value is fresh. It becomes `stale` when a refresh fails but an older value is still usable. Both states include `updatedAtMs`.

Changing the response type invalidates an incompatible cached value. Call `reload()` to bypass freshness and replace the current request.

## Transform a response

Use `map()` to turn a validated response into your app or module's domain model. Use `mapError()` to expose a smaller tagged error contract.

```tsx
const forecast = cachedJson<ProviderForecast>(url, {
  maxAgeMs: 300_000,
}).map((response) => ({
  temperature: response.current.temperature_2m,
  summary: weatherSummary(response.current.weather_code),
})).mapError(toWeatherError);
```

Mapping functions must use Ink's pure TypeScript subset. They can construct objects, map and filter lists, narrow tagged values, and call other pure package functions. They cannot perform I/O, mutate state, read the clock, or call a native capability.

The resource keeps its original loading, stale, cancellation, and reload behaviour. A mapping failure returns an `invalid-response` error unless `mapError()` translates it.

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
  ready: ({ value }) => <Text>Saved {value.id}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

Ink reads state values when `run()` starts. Calling `run()` while the mutation is already running has no effect. Mutations are not retried automatically because repeating a write may not be safe.

## Authenticate a request

Pass an opaque authorisation from `@ink/auth` to a request:

```tsx
if (account.status === "signed-in") {
  const profile = json<Profile>("https://api.example/profile", {
    authorization: account.value.authorization,
  });
}
```

For a manually supplied bearer token, save it with `@ink/secure-store` and pass `bearer(saved.secret)`. Ink never exposes the restored token to app code or diagnostics.

Use `sensitiveQuery` when a provider requires a credential in the query string:

```tsx
const response = json<Result>(url, {
  query: { apikey: apiKey.secret },
  sensitiveQuery: ["apikey"],
});
```

Sensitive query values are redacted from logs, errors, cache metadata, and `ink info`. They remain visible to the remote server and can still be extracted from a distributed client app.

## Create a background request plan

Use `getJson<T>()` or `uploadFile()` inside an `@ink/background` task. These functions describe work and do not start a request where they are declared.

```tsx
import { backgroundTask } from "@ink/background";
import { getJson } from "@ink/network";
import { replaceStoredValue } from "@ink/store";

const refresh = backgroundTask({
  key: "forecast.refresh",
  schedule: { kind: "periodic", everyMinutes: 30 },
  work: replaceStoredValue(
    "forecast.latest",
    getJson<Forecast>("https://weather.example/forecast"),
  ),
});
```

Background request plans use literal URLs, query fields, headers, and response types. `uploadFile()` also requires an idempotency key or an endpoint marked safe to repeat. Read [Background work](background.md) for scheduling and retry behaviour.

The compiler rejects direct `Authorization`, `Cookie`, and proxy credential headers. Authorisation is removed from cross-origin redirects.

## Lifecycle and errors

Reads are screen-scoped. Leaving the screen or calling `reload()` cancels the active request and ignores any late result. Identical active GET requests share one in-flight operation.

GET requests retry one transient connection or server failure within the original timeout. Redirects are limited to five and cannot downgrade from HTTPS to HTTP.

Errors distinguish offline access, timeout, unauthorised and forbidden requests, missing resources, rate limits, server failures, invalid responses, oversized responses, unavailable networking, and unexpected failures. Every error provides `kind`, `message`, and `retryable`. Rate-limit errors also provide `retryAtMs` when the server supplies a valid retry time.
