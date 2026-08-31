# Data and effects

Ink keeps asynchronous work explicit. Reads are resources, writes are mutations, and both are screen-scoped unless they are declared in the application root.

## Data types

Resource and persisted-state types may contain numbers, booleans, strings, `null`, literal unions, lists and objects. Object fields may be optional.

```ts
type Forecast = {
  unit: "celsius" | "fahrenheit";
  temperature: number;
  description?: string;
  warning: string | null;
};
```

The compiler emits a schema from the TypeScript type and validates native or network data before it reaches the UI.

## Cached reads

`cachedJson` stores a validated response in the application's private cache. Fresh data is returned without a request. If a refresh fails, an older response remains available through the `stale` branch for the configured window.

```tsx
import { cachedJson } from "@ink/network";
import { Screen, Text, match } from "ink";

type Weather = { temperature: number };

export default function Weather() {
  const weather = cachedJson<Weather>("https://example.com/weather", {
    maxAgeMs: 300_000,
    staleIfErrorMs: 86_400_000,
  });

  return (
    <Screen title="Weather">
      {match(weather, {
        loading: () => <Text>Loading...</Text>,
        ready: (result) => <Text>{result.value.temperature}</Text>,
        stale: (result) => <Text>{result.value.temperature}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
    </Screen>
  );
}
```

Cache policy never changes an ordinary `json` read. Both APIs retain cancellation, timeout and single-flight behaviour.

## Mutations

A mutation is idle until `run()` is pressed. It is never retried automatically because Ink cannot infer whether a write is safe to repeat.

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
        idle: () => <Button onPress={() => save.run()}>Save</Button>,
        running: () => <Text>Saving...</Text>,
        ready: (result) => <Text>Saved {result.value.id}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
    </Screen>
  );
}
```

Mutation bodies are compiled JSON templates. Scalar state values are materialised at the moment the request starts.

## Composition and computed values

`all` is a strict, fail-fast gate over ordinary async reads. It is ready only when every member is ready and exposes a typed object of their values.

```tsx
const page = all({ weather, airQuality });

{match(page, {
  loading: () => <Text>Loading...</Text>,
  ready: (result) => <Text>{result.value.weather.temperature}</Text>,
  error: () => <Text>Could not load this page</Text>,
})}
```

`computed` lowers a pure scalar expression into the native value graph. It supports scalar state, resource, controller and route values with `+`, `-`, `*` and `/`.

```tsx
const count = state(2);
const doubled = computed(() => count.value * 2);

<Text>{doubled.value}</Text>
```

Neither helper adds a scheduler or a JavaScript runtime.

## Route data

Route data belongs to the navigation entry rather than global state. The compiler checks object `href` values against the target screen's declared `routeParams<T>()` contract, including across separate screen files.

```tsx
// Sender
<Button href={{ path: "/forecast", params: { city: "London" } }}>
  Forecast
</Button>

// /forecast screen
const params = routeParams<{ city: string }>();

<Text>{params.city}</Text>
```

Optional fields may be omitted. Unknown, missing or incorrectly typed route fields are compile errors.

## Visibility

`ink info` reports the native modules, permissions, resources, controllers and state included by an application. `ink logs --resources` shows native request starts, cancellations, outcomes and elapsed time without the rest of Logcat.
