---
title: "Background work"
description: "Durable jobs with fresh JavaScript runtimes and recoverable inputs."
tag: "Design specification"
---

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

The build bundles registered task code and its dependencies into a headless entry. Enqueueing persists the task ID and JSON input; it does not serialise a function or capture foreground state. Imports and normal TypeScript logic work inside workers.

## Execution contract

Each invocation gets a fresh runtime, cancellation signal and decoded input. Open storage and account services there. Return `success`, `retry` with an optional requested delay, or `failed` with a stable reason. An uncaught exception is recorded as a failure; the app must deliberately classify retryable failures.

Android chooses when eligible jobs run. Periodic work has a minimum interval of 15 minutes and is inexact. Network and charging constraints narrow eligibility; they do not guarantee an execution time. A process may stop before recording completion, so delivery is at least once.

A key identifies unique scheduled work: enqueueing the same key coalesces a pending request. If it is already running, a follow-up run is retained so newly queued data is not lost. Keys are not substitutes for idempotency at the remote service.

## Persist before scheduling

Commit an outbox row before requesting a wakeup. Recover unscheduled rows on app launch and later scheduled runs; a database commit and Android scheduling are not one transaction. Send with stable operation IDs, reconcile uncertain acknowledgements and delete or mark an entry only after acceptance.

`cancel(key)` removes pending work and signals running work. Cancellation cannot reverse a completed remote write. Observing job state is useful for a sync screen but must not be required for the job to finish.

## Choose native services for continuous work

Audio playback and downloads have specialised native lifecycles. A permanent JavaScript loop or interval is not a background service. Push can request reconciliation through [LightOS](light-sdk.md) where supported, but handlers must validate the payload and tolerate duplicate delivery.

No React tree is mounted in a worker. Ordinary domain functions and npm libraries matching the host profile can run there.
