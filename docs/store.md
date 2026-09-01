---
title: "Store"
description: "Persist typed app values and cache disposable data."
tag: "Planned"
---

`@ink/store` saves non-sensitive typed values across app restarts and keeps bounded cache entries that your app can recreate.

## Persist a value

Use `storedValue<T>()` when a screen needs to observe loading, saving, or storage errors.

```tsx
import { storedValue } from "@ink/store";
import { Button, Text, match } from "ink";

type Settings = {
  unit: "celsius" | "fahrenheit";
  alerts: boolean;
};

const settings = storedValue<Settings>("settings", {
  unit: "celsius",
  alerts: false,
});

{match(settings, {
  loading: () => <Text>Loading settings</Text>,
  ready: ({ value }) => <Text>{value.unit}</Text>,
  saving: ({ value }) => <Text>Saving {value.unit}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}

<Button onPress={() => settings.set({
  unit: "fahrenheit",
  alerts: false,
})}>
  Use Fahrenheit
</Button>
```

`set()` replaces the complete value. Calls for one key are saved in order, and the latest complete value wins. `reset()` restores the declared initial value and removes the saved record. `retry()` repeats the failed read, write, or removal.

Declarations with the same key share one app-wide value and must use the same type and initial value. Keys must match `[A-Za-z0-9][A-Za-z0-9._-]{0,127}`.

Use core `persistedState()` for scalar and list state that does not need a visible persistence status:

```tsx
import { persistedState } from "ink";

const temperatureUnit = persistedState("settings.temperature", "celsius");
```

## Cache a value

Use `cachedValue<T>()` for data that may be removed at any time and regenerated.

```tsx
import { cachedValue } from "@ink/store";
import { Button, Text, match } from "ink";

const suggestions = cachedValue<ReadonlyArray<string>>("search.suggestions", {
  maxAgeMs: 3_600_000,
});

<Button onPress={() => suggestions.put(["London", "Paris"])}>
  Save suggestions
</Button>

{match(suggestions, {
  loading: () => <Text>Loading suggestions</Text>,
  empty: () => <Text>No saved suggestions</Text>,
  ready: ({ value }) => <Text>{value}</Text>,
  stale: ({ value }) => <Text>{value}</Text>,
  saving: ({ value }) => <Text>{value}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

A cache entry is `ready` until `maxAgeMs` passes, then becomes `stale`. Both states include `updatedAtMs`. The cache does not refresh its producer. Call `put()` with a replacement value or `remove()` to return the entry to `empty`.

Stored values are limited to 1 MiB each and share an 8 MiB app quota. Cache entries share a 32 MiB quota and use least-recently-used eviction.

## Persist background results

Use `replaceStoredValue()` with an approved `@ink/background` work plan. This example fetches typed JSON and replaces one stored value when Android runs the task:

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

`@ink/background` owns scheduling and retries. The source package validates the result, and store replaces the destination atomically.

## Lifecycle and errors

Stored values and cache entries are app-scoped. The first declaration starts one read, and later declarations reconnect to the same in-memory record.

An incompatible stored schema returns a `schema` error and keeps the declared initial value available. An incompatible cache entry is removed because cache data is disposable. Failed writes never expose a partial record.

Errors distinguish unavailable storage, quota limits, corrupt records, incompatible schemas, storage failures, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.

Do not save credentials, tokens, or private keys in this package. Use `@ink/secure-store` for small secrets and `@ink/files` for user-visible documents or media.
