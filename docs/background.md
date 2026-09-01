---
title: "Background work"
description: "Run constrained deferred and periodic work with Android JobScheduler."
tag: "Partial"
---

`@ink/background` runs declarative work plans after the app leaves the foreground. A task owns its schedule, retry policy, and latest durable result without running app JavaScript.

## Create a periodic task

Give a static work plan a stable key:

```tsx
import { backgroundTask } from "@ink/background";
import { getJson } from "@ink/network";

type Forecast = { temperature: number; summary: string };

const refresh = backgroundTask({
  key: "forecast.refresh",
  schedule: { kind: "periodic", everyMinutes: 30 },
  constraints: { network: "connected" },
  work: getJson<Forecast>("https://example.com/forecast.json"),
});
```

`refresh` is a `Resource<Forecast, BackgroundError>`. Its latest successful value survives process death. A ready value includes `completedAtMs`, `freshness`, and an optional `warning` when a later run failed.

Declarations that reuse a key reconnect to the same task and must use the same work and result type. Package-created keys are automatically namespaced to the package.

`everyMinutes` accepts 15 to 10,080 minutes. Android chooses the exact run time based on network availability, power, and system scheduling. Periodic work is not an exact alarm.

## Run deferred work

Use a deferred schedule for work enqueued by a user action:

```tsx
const upload = backgroundTask({
  key: "logs.upload",
  schedule: { kind: "deferred" },
  constraints: {
    network: "unmetered",
    charging: true,
  },
  work: uploadManagedFile(logs.reference, "https://example.com/logs", {
    idempotencyKey: "logs.current.v1",
  }),
});

<Button onPress={() => upload.enqueue()}>Upload later</Button>
```

Calling `enqueue()` replaces pending work with the same key. Call `cancel()` to remove pending work. Cancellation cannot undo a request that the server already accepted.

A deferred task exposes its result resource plus `enqueue()` and `cancel()`. It does not add separate `enqueued` or `running` rendering branches. Use `activity` when the foreground process is observing an active run.

## Choose constraints

| Option | Values | Default |
| --- | --- | --- |
| `network` | `"none"`, `"connected"`, `"unmetered"` | `"none"` |
| `charging` | `boolean` | `false` |
| `batteryNotLow` | `boolean` | `true` |
| `storageNotLow` | `boolean` | `true` |

Android can delay work after every constraint becomes true. The task keeps no wake lock while it waits.

## Build a work plan

Background tasks accept serialisable plans exported by approved Ink modules. A plan is not a callback. It can:

- call approved source operations;
- invoke a declared native worker;
- sequence bounded steps;
- transform a serialisable result with Ink's pure-expression subset;
- use durable file, credential, and record references granted to the task.

A plan cannot read screen state, navigate, render UI, access a screen-owned session, or execute arbitrary TypeScript. Materialise permitted inputs when you enqueue deferred work.

Extension modules can expose domain plans such as `OrdersSync.plan(...)`. Ink still owns scheduling, retries, durable result state, and lifecycle.

## Use durable inputs

Temporary `FileHandle` values cannot enter background work. Pass a durable `FileReference` from Files or Downloads. Ink validates that the reference remains owned by the app before each attempt.

Credentials use opaque background-safe references. Their readable values are resolved only inside the operation that has been granted access. Expired or removed credentials settle as `authentication-required` without exposing the value.

Every repeatable mutation or upload must declare an idempotency key or use an operation whose interface guarantees safe repetition.

## Retry failed work

Ink retries retryable connection and server failures with bounded exponential backoff. One run makes at most three attempts within its Android execution window.

Validation, permission, authentication, quota, integrity, and non-idempotent operation errors are not retried automatically. Call `enqueue()` after the app resolves the cause.

Periodic tasks keep their next scheduled run after one run fails. A previous successful value remains ready with the failure in `warning`.

## Lifecycle and limits

Tasks are application-scoped. An app can declare up to 16 keys. One serialised result is limited to 1 MiB, and one run can execute for up to 10 minutes. Android can impose stricter limits.

Changing a task's work or result type invalidates its previous success. Changing only scheduling or constraints preserves it. Removed declarations are reconciled the next time the updated app starts; uninstalling removes all tasks and results.

Rust owns state, ordering, cancellation, and durable results. Android owns JobScheduler execution. Cancellation stops observation immediately; stopping already-running native or network work is best effort.

## Errors and permissions

Errors distinguish unavailable scheduling, authentication, network and timeout failures, invalid results, expired inputs, storage failures, quota limits, and unexpected failures. Every error provides `kind`, `message`, `retryable`, `operation`, and `attemptedAtMs`.

The module requests no runtime permission. A work plan contributes only the network, file, credential, worker, or other operations it reaches. `ink info` explains each linked requirement.

Background work does not post a notification implicitly. A domain plan can explicitly include a declared notification operation when notifying the user is part of its interface.
