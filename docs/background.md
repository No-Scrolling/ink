# Background resources

`@ink/background` provides install-owned, periodic HTTPS JSON reads. A declaration registers one persisted Android `JobScheduler` job and exposes its latest durable result only while the declaring screen is active.

```tsx
import { periodicJson } from "@ink/background";

type Forecast = { temperature: number; summary: string };

const forecast = periodicJson<Forecast>(
  "forecast",
  "https://example.com/forecast.json",
  { everyMinutes: 30, timeoutMs: 15_000 },
);
```

The resource starts as `waiting`. A successful run produces `ready` with `value` and `updatedAtMs`. A later failure preserves the last success as `stale`; a failure before any success produces `error`. Errors report `kind`, `message`, `retryable` and `attemptedAtMs`.

Intervals are 15 to 10,080 minutes and timeouts are 1,000 to 120,000 milliseconds. URLs must use HTTPS. Query values and headers must be literals, responses are limited to 1 MiB, and decoded JSON must match the declared TypeScript data type. An app may own at most 16 background keys. Repeated declarations of one key must be identical.

Changing a URL, query, headers or response type invalidates the previous value. Changing only the cadence or timeout preserves it. Android controls the exact execution time according to network availability and system scheduling policy.

This API deliberately does not run application callbacks, mutate Ink state or post notifications. It is intended for durable polling whose result can be observed when a screen is shown.
