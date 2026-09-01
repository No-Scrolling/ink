---
title: "Data and effects"
description: "Model typed resources, actions, sessions, caching, and composition."
---

Ink represents asynchronous reads as resources, explicit work as actions, and long-lived capabilities as sessions. Their tagged states can be narrowed with `match`.

Resources use `loading`, `ready`, and `error`. A usable cached value remains `ready`; `freshness`, `activity`, and `warning` describe background refresh without forcing another rendering branch. Actions use `idle`, `running`, `success`, and `error`. Sessions expose one domain state snapshot plus ordered commands.

## Supported data types

Resource responses, route data, and persisted state can contain:

- `number`, `boolean`, `string`, and `null` values;
- literal unions;
- lists;
- nested objects with required or optional fields.

```ts
type Forecast = {
  unit: "celsius" | "fahrenheit";
  temperature: number;
  description?: string;
  warning: string | null;
};
```

Ink generates a schema from the TypeScript type. Native and network data must match that schema before it reaches the UI.

## Read JSON

Use `json<T>()` for an HTTPS GET request. The resource begins loading when its screen becomes active.

```tsx
import { json } from "@ink/network";
import { Screen, Text, match } from "ink";

type Weather = { temperature: number };

export default function Weather() {
  const weather = json<Weather>("https://example.com/weather", {
    query: { city: "London" },
    timeoutMs: 15_000,
  });

  return (
    <Screen title="Weather">
      {match(weather, {
        loading: () => <Text>Loading weather</Text>,
        ready: (result) => <Text>{result.value.temperature}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
    </Screen>
  );
}
```

Call `reload()` to replace the current request. Ink cancels the old request and ignores any late result.

URLs must use HTTPS. Query values can be `string`, `number`, or `boolean` literals, lists of those values, or compatible state values. Lists are encoded as comma-separated values. Header values can be string literals or string state values. `timeoutMs` accepts 1,000 to 120,000 milliseconds and defaults to 15 seconds.

## Cache a read

Pass `cache` to `json<T>()` when a response can be reused across app launches.

```tsx
const weather = json<Weather>("https://example.com/weather", {
  cache: {
    freshForMs: 300_000,
    staleIfErrorForMs: 86_400_000,
  },
});
```

| Option | Meaning | Default |
| --- | --- | --- |
| `freshForMs` | Return the saved value without a request while it is this fresh. | 5 minutes |
| `staleIfErrorForMs` | Keep an older value available when a refresh fails. | 1 day |

A cached ready result adds metadata:

- `freshness` is `"fresh"` or `"stale"`.
- `activity` is `"idle"` or `"refreshing"`.
- `updatedAtMs` records when the value was saved.
- `warning` contains the latest refresh error while an older value remains usable.

Changing the response schema invalidates an incompatible saved value.

## Send a mutation

Use `mutation<T>()` for a `POST`, `PUT`, `PATCH`, or `DELETE` request. A mutation remains `idle` until you call `run()`.

```tsx
import { mutation } from "@ink/network";
import { Button, Screen, Text, match, state } from "ink";

type Saved = { id: string };

export default function Save() {
  const name = state("London");
  const save = mutation<Saved>("https://example.com/locations", {
    method: "POST",
    body: { name: name.value },
  });

  return (
    <Screen title="Save">
      {match(save, {
        idle: () => <Button onPress={() => save.run()}>Save location</Button>,
        running: () => <Text>Saving location</Text>,
        success: (result) => <Text>Saved {result.value.id}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
    </Screen>
  );
}
```

Mutation options accept:

- `method` as `"POST"`, `"PUT"`, `"PATCH"`, or `"DELETE"`;
- query values and headers from literals or compatible scalar state;
- a JSON body containing literals and scalar state values;
- `timeoutMs` from 1,000 to 120,000 milliseconds.

Ink materialises state values when `run()` starts. Mutations are not retried automatically because a repeated write might not be safe.

## Combine resources

Use `all()` when a screen needs several resources before it can render.

```tsx
const page = all({ weather, airQuality });

{match(page, {
  loading: () => <Text>Loading conditions</Text>,
  ready: (result) => <Text>{result.value.weather.temperature}</Text>,
  error: (result) => (
    <Text>{result.error.resource}: {result.error.error.message}</Text>
  ),
})}
```

The combined resource is ready when every member has a usable ready value. It preserves freshness and warnings from each member. Its error identifies the failed member and preserves that member's structured error.

## Compute a value

Use `computed()` for a scalar value derived from state, route data, resources, or controllers.

```tsx
const count = state(2);
const doubled = computed(() => count.value * 2);

<Text>{doubled.value}</Text>
```

Computed expressions support scalar values and `+`, `-`, `*`, and `/`. Ink compiles the expression into its native value graph.

## Pass route data

Route data belongs to a navigation entry instead of global state. Pass scalar fields in an object `href`, and declare the destination contract with `routeParams<T>()`.

```tsx
// Source screen
<Button href={{ path: "/forecast", params: { city: "London" } }}>
  London
</Button>

// Destination screen
const params = routeParams<{ city: string }>();

<Text>{params.city}</Text>
```

Ink reports unknown, missing, or incorrectly typed route fields at build time.

## Lifecycle

Resources declared in a screen are active only while that screen is visible. Leaving the screen cancels observation and ignores late results; termination of underlying work is best effort. Reloading uses single-flight ordering, so an older completion cannot replace a newer request.

Opaque handles can connect modules without exposing their contents. They cannot enter state, route data, persisted values, or ordinary serialised results. Rust owns their generation and lifetime. An operation that needs to survive process death must use a durable reference rather than a screen-owned handle.

Errors provide `kind`, `message`, and `retryable`. Use `ink logs --resources` to inspect request starts, cancellations, results, and elapsed time. Use `ink info` to list the app's resources and native capabilities.
