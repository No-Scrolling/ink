---
title: "Data and lifecycle"
description: "Async functions, resource hooks, actions and explicit cancellation."
tag: "Design specification"
---

Start with ordinary functions. A weather module can return `Promise<Forecast>`; the same function can be called by a screen or a background worker. Ink's hooks add UI state and ownership with explicit lifecycle management.

## Load a value

```tsx
import { Button, Screen, Text, useResource } from "ink";
import { getForecast } from "./weather";

export default function ForecastScreen({ placeId }: { placeId: string }) {
  const forecast = useResource(
    ["forecast", placeId],
    ({ signal }) => getForecast(placeId, { signal }),
    { staleTime: 300_000 },
  );

  return (
    <Screen title="Forecast">
      {forecast.status === "loading" && <Text>Loading forecast</Text>}
      {forecast.status === "error" && <Text>{forecast.error.message}</Text>}
      {forecast.status === "ready" && <Text>{forecast.data.temperature}°</Text>}
      <Button onPress={forecast.reload} disabled={forecast.refreshing}>Refresh</Button>
    </Screen>
  );
}
```

The key is a structurally compared array of JSON values. Include every input that identifies the result: account, location, units and filters. Changing the function's identity does not refetch; changing the key does. Returning to a visible screen refreshes stale data.

`useResource` starts after commit while the screen is visible. It aborts replaced work and ignores late completions. A shared read is cancelled only after its last active observer releases it. Functions sharing a key must implement the same operation and data contract.

| State | Fields |
| --- | --- |
| `loading` | No usable value yet. |
| `ready` | `data`, `updatedAt`, `refreshing`, optional `warning`. |
| `error` | `error`; no usable value. |

`refreshing` and `reload` are available in every state; `refreshing` is false when no read is running. A failed refresh keeps a previous value in `ready` with a warning. `reload()` bypasses freshness and supersedes the previous read. An explicit `enabled: false` option suppresses work; without cached data the state is `idle`, so render that branch when using the option.

The shared cache is in memory and bounded. `staleTime` is a freshness policy, not durable storage. Use [Store](store.md) or [Records](records.md) when offline availability is part of the product. Do not persist personalised data under a cache key that omits the account.

## Run an action

```tsx
import { useState } from "react";
import { Button, Text, useAction } from "ink";
import { savePlace } from "./places";

export function AddPlace() {
  const [name, setName] = useState("London");
  const save = useAction((value: string, { signal }) => savePlace(value, { signal }));
  return (
    <>
      <Button disabled={save.status === "running"} onPress={() => save.run(name)}>Save {name}</Button>
      {save.status === "success" && <Text>Place saved</Text>}
      {save.status === "error" && <Text>{save.error.message}</Text>}
    </>
  );
}
```

`useAction` has `idle`, `running`, `success` and `error` states, with `data` on success. `run(input)` uses the current input and returns a promise that resolves to `{ ok: true, value }` or `{ ok: false, error }`; event handlers may ignore that result because the hook retains it. Concurrent calls join the running attempt by default, even if a later call supplies different input. Disable the control while running; use a domain queue when every input must be retained. `reset()` clears a settled result; `cancel()` signals cancellation and returns the hook to idle. Ordinary actions are cancelled when the component unmounts; hiding a retained screen only stops its UI observation. External-activity operations retain their documented round-trip ownership.

Underlying package functions return ordinary rejecting promises. Ink normalises caught errors for UI, preserving the original cause for diagnostics. Actions do not retry mutations automatically. Aborting cannot undo a message already accepted by a server. A durable outbox belongs in a domain module, not a component hook.

## Validate external values

TypeScript generics are erased. Neither `response.json()` nor `useResource<Forecast>()` validates a server response. Start with `unknown` and decode using a normal function or a schema library you choose. Ink does not include a schema framework by default.

Native bindings validate their transport contract on both sides. JSON storage also needs explicit decoders and versioned migrations. Those checks establish structure, not the truth of provider data.

## Cancel work

Async functions accept an optional `AbortSignal` when cancellation is meaningful. Pass the supplied signal through nested requests. An operation must release its native observation promptly after abort; native completion and remote side effects can still happen later.

```tsx
useVisibleEffect(() => {
  const controller = new AbortController();
  const unsubscribe = messages.subscribe(roomId, updateMessages, {
    signal: controller.signal,
  });
  return () => {
    controller.abort();
    unsubscribe();
  };
}, [roomId]);
```

This fragment assumes a domain module and state setter supplied by the screen. A subscription must not silently drop message deltas: persist and reconcile them, or expose a gap requiring a fresh snapshot.

A native picker or browser sign-in owns an external activity round trip. Its operation-specific lifecycle keeps it alive through that temporary pause. Ordinary backgrounding does not grant indefinite execution.

## Observe ongoing state

A store's `getSnapshot()` returns an immutable snapshot and `subscribe(listener)` returns an unsubscribe function. Snapshot identity changes only when its content changes. `useSnapshot(store)` adapts this contract through React’s external-store subscription semantics, subscribes while visible and reads the latest snapshot again on return. A shared source releases native observation only when its last active subscriber leaves.

Native UI hooks such as `usePlayer()` or `useScanner()` manage activation, expose `state` plus promise-returning commands, and release their native attachment when hidden. Constructing a controller during render must be inert; acquisition happens after commit. The package owns this hook integration, so an app does not write a session manager for each screen.

Continuous state, such as playback position, can coalesce. Ordered events, such as incoming messages or protocol packets, need bounded queues with an explicit overflow policy. A current snapshot and a delivery queue are different contracts.

## Errors and recovery

Native package errors extend `Error` and expose a stable `code`, operation and optional cause. Useful codes include `permission-denied`, `unavailable`, `cancelled`, `timeout`, `closed` and domain-specific failures. Use exported error guards to narrow a caught `unknown`.

Do not infer a retry policy from every error having a boolean. Retry a read when useful; retry a write only when the domain has an idempotency or reconciliation strategy.
