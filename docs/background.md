---
title: "Background work"
description: "Run constrained deferred and periodic work with Android JobScheduler."
---

`@ink/background` runs declarative work plans after the app leaves the foreground. Background tasks can fetch typed data and persist a result without running app JavaScript.

## Create a periodic task

Compose a network plan with a store destination, then give it a stable key:

```tsx
import { backgroundTask } from "@ink/background";
import { getJson } from "@ink/network";
import { replaceStoredValue } from "@ink/store";

type Forecast = { temperature: number; summary: string };

const refresh = backgroundTask({
  key: "forecast.refresh",
  schedule: { kind: "periodic", everyMinutes: 30 },
  constraints: { network: "connected" },
  work: replaceStoredValue(
    "forecast.latest",
    getJson<Forecast>("https://example.com/forecast.json"),
  ),
});
```

The task declaration is app-scoped. Declarations that reuse a key must use the same work plan and destination.

`everyMinutes` accepts 15 to 10,080 minutes. Android chooses the exact run time based on network availability, power, and system scheduling. Periodic work is not an exact alarm.

## Run deferred work

Use a deferred schedule for work that should run once when its constraints are met:

```tsx
import { uploadFile } from "@ink/network";

const upload = backgroundTask({
  key: "logs.upload",
  schedule: { kind: "deferred" },
  constraints: {
    network: "unmetered",
    charging: true,
  },
  work: uploadFile(logFile, "https://example.com/logs"),
});

<Button onPress={() => upload.enqueue()}>Upload later</Button>
```

Calling `enqueue()` replaces pending deferred work with the same key. Call `cancel()` to remove pending work. Cancellation cannot undo a request that the server has already accepted.

## Choose constraints

`constraints` accepts:

| Option | Values | Default |
| --- | --- | --- |
| `network` | `"none"`, `"connected"`, `"unmetered"` | `"none"` |
| `charging` | `boolean` | `false` |
| `batteryNotLow` | `boolean` | `true` |
| `storageNotLow` | `boolean` | `true` |

Android can delay work after every constraint becomes true. The task keeps no wake lock while it waits.

## Build a work plan

Background tasks accept plans exported by approved Ink packages. Plans describe native work; they are not callbacks.

The first release supports:

- `replaceStoredValue(key, getJson(...))` for typed HTTPS reads;
- `putCachedValue(key, getJson(...))` for disposable cached data;
- `uploadFile(file, url)` for an idempotent file upload.

`getJson()` validates the response before the store changes. `uploadFile()` requires an idempotency key or an endpoint declared safe to repeat.

A plan cannot read screen state, navigate, render UI, access a controller, or run arbitrary TypeScript. Materialise any permitted input when you enqueue deferred work.

## Observe task state

The value returned by `backgroundTask()` reports the latest durable state:

| Status | Meaning |
| --- | --- |
| `waiting` | The task has not completed a run. |
| `enqueued` | Deferred work is waiting for Android. |
| `running` | The current process is observing an active run. |
| `ready` | The latest run succeeded. |
| `stale` | A later run failed after an earlier success. |
| `error` | A run failed before any success was stored. |

`ready` and `stale` include `completedAtMs`. `stale` and `error` include the latest structured error and `attemptedAtMs`.

Task state survives process death and becomes available when any declaring screen is active. It does not stream progress while the app process is absent.

## Retry failed work

Ink retries retryable network and server failures with bounded exponential backoff. The initial delay is at least 30 seconds, and one run makes at most three attempts within its Android execution window.

Validation, permission, authentication, quota, and non-idempotent operation errors are not retried automatically. Call `enqueue()` after the app resolves the cause.

Periodic tasks keep their next scheduled run after one run fails. Deferred tasks settle as `error` after their retry budget ends.

## Lifecycle and limits

An app can declare up to 16 task keys. One background response is limited to 1 MiB, and one run can execute for up to 10 minutes. Android may impose stricter limits.

Changing a task's work plan or result schema invalidates its previous success. Changing only scheduling or constraints preserves the saved result.

Removing a task declaration cancels future work during the next app update. Uninstalling the app removes all tasks and results.

## Errors and permissions

Errors distinguish unavailable scheduling, unsatisfied authentication, network and timeout failures, invalid responses, storage failures, quota limits, expired inputs, and unexpected failures. Every error provides `kind`, `message`, `retryable`, `operation`, and `attemptedAtMs`.

The module requests no runtime permission. A work plan contributes the network, file, or other capabilities it uses. `ink info` shows each task, constraint, destination, and linked Android component.

Background work cannot post a notification as a side effect. Use [Notifications](notifications.md) for a user-visible reminder.
