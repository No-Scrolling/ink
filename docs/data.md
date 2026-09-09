---
title: "Resources and actions"
description: "Keep fetched data available across screens."
---

## Cached data

This example assumes your `api.ts` exports `getForecast`, which returns a promise containing a temperature.

Use `resource` for asynchronous reads that should survive tab changes. Define resources outside components, then read them with `useSnapshot`:

```tsx
import { resource, useSnapshot, Screen, Text, Button, LoadingState, ErrorState } from "ink";
import "@ink/network";
import { getForecast } from "./api";

const forecast = resource({
  key: (latitude: number, longitude: number, unit: string) =>
    [latitude, longitude, unit],
  load: (latitude, longitude, unit) => getForecast(latitude, longitude, unit),
  staleTime: 60_000,
  refreshInterval: 60_000,
});

function Forecast({ latitude, longitude, unit }: {
  latitude: number; longitude: number; unit: string;
}) {
  const source = forecast(latitude, longitude, unit);
  const result = useSnapshot(source);
  return <Screen title="Forecast">
    {result.status === "loading" ? <LoadingState /> :
      result.status === "error" ? <ErrorState message={result.error.message} onRetry={source.refresh} /> :
      <>
        <Text>{result.data.temperature}°</Text>
        {result.refreshError && <>
          <Text>{result.refreshError.message}</Text>
          <Button onPress={source.refresh}>Try again</Button>
        </>}
      </>}
  </Screen>;
}
```

Calling a resource selects a stable cache entry without loading data. Subscribing starts the request. Keys are arrays of strings, finite numbers, booleans or `null`; include every argument that changes the result. Each resource definition has its own cache.

`staleTime` is how long a successful result stays fresh, in milliseconds. It defaults to zero. Stale data remains visible while a new subscription refreshes it. `refreshInterval` separately enables polling while subscribed; omit it to disable polling. Both intervals are measured from the last successful load.

A ready snapshot contains `data`, `refreshing` and `refreshError`. A failed background refresh keeps the data and sets `refreshError`; a failed initial load produces an error snapshot. Failures stop polling and delay subscription-triggered retries for one minute. `source.refresh()` bypasses freshness and retry delays, sharing any pending request. It resolves when the request settles; failures appear in the snapshot.

Hidden tabs unsubscribe, stopping polling when no visible consumers remain. Putting the app in the background is not a route change and does not itself unsubscribe the screen. Polling is not a background-job guarantee. Pending requests finish into the cache. Unused entries expire after five minutes without subscribers, or five minutes after pending work finishes. The cache is in memory and does not survive app restarts; use [Store](/store) for persistent data. Resources do not install networking globals—import `@ink/network` when the loader uses `fetch`.

Use `useAction` for commands such as saving or sending, and normal effects for live subscriptions such as microphone capture.

