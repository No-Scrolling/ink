# Data and effects

Ink represents asynchronous reads as resources and explicit writes as mutations. Both use tagged status values that TypeScript can narrow with `match`.

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

URLs must use HTTPS. Query values can be `string`, `number`, or `boolean` literals or scalar state values. Header values can be string literals or string state values. `timeoutMs` accepts 1,000 to 120,000 milliseconds and defaults to 15 seconds.

## Cache a read

Use `cachedJson<T>()` when a response can be reused across app launches.

```tsx
const weather = cachedJson<Weather>("https://example.com/weather", {
  maxAgeMs: 300_000,
  staleIfErrorMs: 86_400_000,
});
```

| Option | Meaning | Default |
| --- | --- | --- |
| `maxAgeMs` | Return the saved value without a request while it is this fresh. | 5 minutes |
| `staleIfErrorMs` | Keep an older value available when a refresh fails. | 1 day |

A cached resource adds two fields and one status:

- `updatedAtMs` records when the value was saved.
- `stale` provides the saved `value`, `updatedAtMs`, and the latest refresh `error`.

Changing the response schema invalidates an incompatible saved value. Cache settings do not change ordinary `json()` reads.

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
        ready: (result) => <Text>Saved {result.value.id}</Text>,
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

Use `all()` when a screen needs several ordinary resources before it can render.

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

The combined resource is ready only when every member is ready. Its error identifies the failed member and preserves that member's structured error.

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

## Resource lifecycle

Resources declared in a screen are active only while that screen is visible. Leaving the screen cancels active work. Reloading uses single-flight ordering, so an older completion cannot replace a newer request.

Errors provide `kind`, `message`, and `retryable`. Use `ink logs --resources` to inspect request starts, cancellations, results, and elapsed time. Use `ink info` to list the app's resources and native capabilities.
