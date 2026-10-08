---
title: "Background work"
description: "Schedule work to run after the app closes."
---

Use `@ink/background` to schedule work after the app closes.

## Define a task

Define tasks in a worker file, separate from your UI:

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

`decodeAccountInput` must validate `{ accountId: string }`, including input saved by older app versions. `drainOutbox` is your message-sending function.

Register the worker file in `ink.toml`:

```toml
[background]
entry = "./workers.ts"
```

## Schedule a task

Import the task into your app and enqueue it:

```ts
import { syncMessages } from "./workers";

await syncMessages.enqueue({ accountId }, {
  key: `messages.sync:${accountId}`,
  constraints: { network: "connected" },
});
```

Ink bundles the worker separately from the UI. Scheduling saves the task ID and JSON input, not a function or the screen’s state.

## How tasks run

Each run starts a fresh JavaScript runtime with decoded input and a cancellation signal. Open storage and account services inside the worker.

### Results and retries

Return `success`, `retry` with an optional delay, or `failed` with a reason. Uncaught exceptions are logged as failures; return `retry` for errors that should be tried again.

### Timing and duplicate runs

Android chooses when tasks run. Periodic tasks have a minimum interval of 15 minutes. Network and charging constraints do not guarantee a start time. A task can run more than once if the process stops before saving completion.

Scheduling the same `key` combines pending requests. If the task is already running, Ink keeps a follow-up run. The server must still handle repeated requests safely.

### Limits and available APIs

Workers run for up to two minutes and accept at most 8 KiB of encoded input. Retry delays set the earliest retry time, not a deadline.

Workers can use:

- `fetch`, including response streams and multipart uploads, and `WebSocket`.
- Store and task scheduling or state.
- Notification scheduling, cancellation and status.
- One-off location reads with permission already granted.

Import `"@ink/network"` in workers that use web globals. Workers do not inherit the foreground app’s imports.

Permission prompts, UI controllers, camera, microphone and NFC are unavailable. Native calls have their own timeouts and are cancelled when the worker stops.

## Persist before scheduling

Save outgoing changes before scheduling a task. Recover any unscheduled changes when the app opens or a task runs: saving data and scheduling can fail separately.

Give each operation a stable ID so the server can recognise retries. Mark it complete only after the server confirms acceptance.

## Cancel a task

Cancel using the key passed when scheduling:

```ts
import { cancel } from "@ink/background";

await cancel(`messages.sync:${accountId}`);
```

Cancellation removes pending work and signals running work to stop. It cannot undo a completed server request.

## Observe tasks

Read scheduling information and the latest outcome for up to 256 task keys:

```ts
import { jobs } from "@ink/background";

const currentJobs = await jobs.get();
```

### Watch for changes

Use the shared source in a component. Ink starts observation when subscribed and cancels it when the last subscriber leaves:

```ts
import { jobs } from "@ink/background";
import { useSnapshot } from "ink";

const snapshot = useSnapshot(jobs);
```

The snapshot is `loading`, `ready` with job data, or `error`. For non-React consumers, `watchJobs({ signal })` remains available as an async iterable.

States are `queued`, `running`, `retrying`, `succeeded`, `failed` or `cancelled`. `scheduled` and `periodic` describe pending work, so a periodic task can be both successful and still scheduled.

Task history survives app restarts. Save your app’s progress and results in [Store](/store).

## Continuous work

Use [Audio](/audio) for playback and [Downloads](/downloads) for file transfers. A JavaScript loop cannot keep an app running in the background. [LightOS push](/light-sdk#background-push-tasks) can schedule a task; validate its payload and handle duplicate deliveries.
