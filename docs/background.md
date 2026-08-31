---
title: "Background work"
description: "Fetch and persist JSON periodically with Android JobScheduler."
---

`@ink/background` performs periodic HTTPS JSON reads with Android `JobScheduler`. Results are stored by the app and become available whenever the declaring screen is active.

## Create a periodic resource

```tsx
import { periodicJson } from "@ink/background";
import { Screen, Text, match } from "ink";

type Forecast = { temperature: number; summary: string };

const forecast = periodicJson<Forecast>(
  "forecast",
  "https://example.com/forecast.json",
  { everyMinutes: 30, timeoutMs: 15_000 },
);

<Screen title="Forecast">
  {match(forecast, {
    waiting: () => <Text>Waiting for an update</Text>,
    ready: (result) => <Text>{result.value.temperature}</Text>,
    stale: (result) => <Text>{result.value.temperature}</Text>,
    error: (result) => <Text>{result.error.message}</Text>,
  })}
</Screen>
```

The key identifies the durable value. Declarations that reuse a key must use the same URL, options and response type.

## Resource states

| Status | Meaning |
| --- | --- |
| `waiting` | No background run has completed. |
| `ready` | The latest run succeeded. `value` and `updatedAtMs` are available. |
| `stale` | A refresh failed, but the previous value remains available. |
| `error` | A run failed before any value was saved. |

Errors provide `kind`, `message`, `retryable`, and `attemptedAtMs`.

## Scheduling

`everyMinutes` accepts 15 to 10,080 minutes. Android chooses the exact run time based on network availability, power, and system scheduling. This API is suitable for durable polling, not exact alarms.

`timeoutMs` accepts 1,000 to 120,000 milliseconds. URLs must use HTTPS. Query values and headers are compile-time literals.

## Validation and storage

Ink validates decoded JSON against the declared TypeScript type before saving it. Responses are limited to 1 MiB, and one app can own up to 16 background keys.

Changing the URL, query, headers, or response type invalidates the saved value. Changing only the interval or timeout preserves it.

Background resources do not run app callbacks, mutate Ink state, or post notifications. Use [Notifications](notifications.md) when an app needs a scheduled reminder.
