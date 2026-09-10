---
title: "Resources and actions"
description: "Keep fetched data available across screens."
---

## Cached data

Use `resource` to keep fetched data available across tab changes. Define it outside components and read its state with `useSnapshot`.

This example uses a `getForecast` function from `api.ts` that returns a promise containing a temperature:

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

## Cache keys

Calling a resource selects a cache entry. `useSnapshot` subscribes to that entry and loads it when needed. Each resource definition has its own cache.

Include every argument that changes the result in the key. Keys are arrays of strings, finite numbers, booleans or `null`.

## Refresh intervals

`staleTime` sets how long a result stays fresh, in milliseconds. It defaults to zero. Subscribing to stale data starts a refresh and keeps the previous data visible.

`refreshInterval` refreshes data periodically while subscribed. Omit it to disable polling. Both intervals start from the last successful load.

## Errors and retries

A ready snapshot contains `data`, `refreshing` and `refreshError`. If the first load fails, the snapshot has an error status. If a refresh fails, it keeps the previous data and sets `refreshError`.

Failures stop polling. For one minute after a failure, new subscriptions do not trigger a retry.

Call `source.refresh()` to retry immediately, even if the data is fresh or a retry delay is active. It reuses a pending request and resolves when that request finishes. Read failures from the snapshot.

## Cache lifetime

Hidden tabs unsubscribe. Polling stops when no subscribers remain, but pending requests finish and update the cache. Unused entries expire after five minutes without subscribers, or five minutes after pending work finishes.

Putting the app in the background does not unsubscribe the screen. Resource polling does not guarantee background execution; use [background jobs](/background) for scheduled work.

The cache clears when the app restarts. Use [Store](/store) to save data between sessions.

## Network requests

Import `@ink/network` when the loader uses `fetch`. A resource manages fetched data; it does not provide networking APIs.

## Save and send actions

Use `useAction` to track commands such as saving or sending and display their errors. Use React effects for live subscriptions such as [microphone capture](/permissions-guide).

For a setting, save the value before calling `back()` so the previous page shows the change. Here, `preferences` is a [Store](/store) with a `units` setting:

```tsx
import { back, useAction } from "ink";

const save = useAction(async (units: string) => {
  await preferences.update(current => ({ ...current, units }));
  back();
});
```

Pass `save.run` to the input or selection handler. `useAction` ignores repeated submissions while pending; display `save.error.message` when `save.status === "error"`. Keep inputs visible while saving, with errors underneath.
