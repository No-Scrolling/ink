---
title: "Background work"
description: "Durable jobs with fresh JavaScript runtimes and recoverable inputs."
tag: "In development"
---

> **In development.** Persisted jobs, separate workers, constraints, retries and job-state observation are implemented. Concurrency and reboot recovery need broader verification.

`@ink/background` runs deferrable work such as refreshing saved forecasts, synchronising a feed or draining a message outbox. Register worker code separately from the UI.

```ts
// workers.ts
import { defineTask } from "@ink/background";
import { drainOutbox } from "./messages";

export const syncMessages = defineTask({
  id: "messages.sync",
  decode: decodeAccountInput,
  async run({ input, signal }) {
    await drainOutbox(input.accountId, { signal });
    return { status: "success" };
  },
});
```

`decodeAccountInput` is the app's decoder for `{ accountId: string }`. It receives unknown persisted input, including input queued by an older app version.

```toml
[background]
entry = "./workers.ts"
```

From an action or domain module:

```ts
await syncMessages.enqueue({ accountId }, {
  key: `messages.sync:${accountId}`,
  constraints: { network: "connected" },
});
```

The build bundles registered task code and its dependencies into a headless entry. Enqueueing persists the task ID and JSON input; it does not serialise a function or capture foreground state.

## Execution contract

Each invocation gets a fresh runtime, cancellation signal and decoded input. Open storage and account services there. Return `success`, `retry` with an optional requested delay, or `failed` with a stable reason. An uncaught exception is logged as a failure; the app must deliberately classify retryable failures.

Android chooses when eligible jobs run. Periodic work has a minimum interval of 15 minutes and is inexact. Network and charging constraints narrow eligibility; they do not guarantee an execution time. A process may stop before recording completion, so delivery is at least once.

A key identifies unique scheduled work: enqueueing the same key coalesces a pending request. If it is already running, a follow-up run is retained so newly queued data is not lost. Keys are not substitutes for idempotency at the remote service.

Workers have a two-minute execution limit and accept at most 8 KiB of encoded input. Workers can use fetch (including response streams and multipart bodies), WebSocket, Store, task scheduling/state, notifications (schedule/cancel/status) and one-off location with an existing permission grant. Permission screens, UI controllers, camera, microphone and NFC are unavailable inside a worker. Each native call has its own timeout and is cancelled when the worker stops. Retry delays are minimum delays, not execution deadlines.

## Persist before scheduling

Commit an outbox row before requesting a wakeup. Recover unscheduled rows on app launch and later scheduled runs; a database commit and Android scheduling are not one transaction. Send with stable operation IDs, reconcile uncertain acknowledgements and delete or mark an entry only after acceptance.

`cancel(key)` removes pending work and signals running work. Cancellation cannot reverse a completed remote write. `getJobs()` returns current scheduling information and the latest outcome for up to 256 task keys. `watchJobs({ signal })` observes changes without repeatedly querying from JavaScript:

```ts
import { watchJobs } from "@ink/background";

for await (const jobs of watchJobs({ signal })) {
  updateJobs(jobs);
}
```

States are `queued`, `running`, `retrying`, `succeeded`, `failed` or `cancelled`. `scheduled` and `periodic` describe pending Android work; a periodic task can have a successful last result and still be scheduled. The journal survives app restarts. This is task-level status; persist domain-specific progress and results in Store. Android can postpone or interrupt work, so a scheduled task has no promised start time.

## Choose native services for continuous work

Audio playback and downloads have specialised native lifecycles. A permanent JavaScript loop or interval is not a background service. Push can request reconciliation through [LightOS](light-sdk.md) where supported, but handlers must validate the payload and tolerate duplicate delivery.
