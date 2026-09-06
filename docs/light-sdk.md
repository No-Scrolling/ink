---
title: "LightOS"
description: "Tool entry, host preferences and available system services."
---

Use `@ink/lightos` to read host preferences and permissions or open the LightOS dialler. Import ringtone and push features from their separate entry points. Dialler presentation is user-verified on the LP3; audible ringtones and production push still need device checks.

Enable host integration in the app configuration:

```toml
[lightos]
enabled = true
```

The build registers the app's entry points and required host capabilities. Ink coordinates tool launch, navigation, lifecycle and native services. Integration is versioned; the installed host determines which optional operations are available.

## Preferences and system actions

`@ink/lightos` exposes `getVersion()`, `getPreferences()`, `getKeyboardOptions()`, `getPermission(name)`, `requestPermission(name)` and `openDialler({ phoneNumber })`. Ringtone and push bindings have separate package entry points.

```ts
import { lightos } from "@ink/lightos";

const version = await lightos.getVersion();
const preferences = await lightos.getPreferences(); // hapticsEnabled
const keyboard = await lightos.getKeyboardOptions();
await lightos.openDialler({ phoneNumber });
```

The example assumes a user-selected phone number. Use host operations for the dialler and supported ringtone selection. Restricted services can return unavailable, denied or blocked-by-host outcomes. A local Android permission does not override host policy.

A successful response means the host accepted the request. Check visible and audible results on your target phone.

Hardware keys flow through focus, navigation and active media ownership. Avoid separate app listeners competing with the keyboard or media session for the same key.

## Entry and recovery

Your app can open from the launcher, a link or a notification. Decode external route data and restore saved records on a cold launch. Handle missing or outdated destinations with a useful fallback screen.

`getPreferences()` and `getKeyboardOptions()` read fresh host values. Keyboard options include `emojis`, `displayVoice`, `enableKeyAnimation` and nullable `swipeEnabled` (older hosts omit it). These are snapshots, not subscriptions; refresh them when returning to the app. The host does not expose an account snapshot or general capability discovery. Query individual permissions and handle unavailable operations instead of inferring support from its version.

## Jobs, media and push

[Background jobs](background.md) register named handlers and durable inputs. [Audio](audio.md) keeps detached playback under a native media service and reconnects UI to existing state. Both must recover from Android termination.

`usePush()` from `@ink/lightos/push` exposes registration state, a persisted inbox, and `register`, `retry`, `unregister`, `dismiss` and `clear` commands. Registration uses the configured LightOS UnifiedPush distributor. `register(subscriptionBaseUrl, bearerToken?)` sends `PUT {subscriptionBaseUrl}/{installationId}` with `{ "endpoint": "…" }`; unregister sends `DELETE` to that URL. The subscription server must return a successful HTTP status. The template's loopback URL requires a local subscription server and an ADB reverse connection on port 18080.

The current adapter accepts a versioned notification envelope:

```json
{
  "version": 1,
  "events": [{
    "operation": "show",
    "id": "message-42",
    "groupKey": "conversation-7",
    "title": "New message",
    "body": "Your message is ready",
    "route": "/inbox"
  }]
}
```

Use `operation: "clear"` with an `id` and `groupKey` to remove a notification. New messages replace the same group; the final inbox state determines which notifications appear after a batch. Payloads are limited to 4 KiB and 16 events, with 64 inbox entries and 512 recent event IDs retained for deduplication. Notification taps consume the entry and open its optional route.

`retry()` resumes a failed registration or pending unsubscribe. The requested operation survives app restarts.

Native receivers persist and present notifications. An optional registered background task can process the accepted delivery without opening the UI. Push may be duplicated, delayed or unavailable, so keep a durable sync strategy.

## Development and distribution

When switching devices, use `ink dev --device <serial>` to rebuild for the selected host. An APK configured for the emulator's host does not connect to LightOS on the LP3.

`ink info` reports host integration requirements. A development APK can expose unavailable states on an emulator or device lacking the host service. Inspect actual capabilities on the LP3 rather than inferring them from an emulator.

Installing an APK and qualifying for Light-approved distribution are separate processes. Distribution requirements and restricted service access must be agreed with Light; enabling this configuration does not confer approval.

## Background push tasks

Define the handler in the existing worker entry and register it from the UI with `setPushTask(task)`. Enable the background capability and worker bundle as described in [Background](background.md).

```ts
// workers.ts
import { defineTask } from "@ink/background";
import { decodePushDelivery } from "@ink/lightos/push";

export const processPush = defineTask({
  id: "app.process-push",
  decode: decodePushDelivery,
  async run({ input, signal }) {
    await reconcileInbox(input.messages, input.cancelled, signal);
    return { status: "success" };
  },
});
```

```ts
import { setPushTask } from "@ink/lightos/push";
import { processPush } from "./workers";

await setPushTask(processPush);
// Remove the handler when signing out:
await setPushTask(null);
```

`reconcileInbox` is app code. Each accepted delivery contains new messages and cancelled group keys after native deduplication. Android schedules a persisted task in a fresh worker runtime; the handler has the same two-minute deadline, cancellation signal and native API access as other tasks. This is deferrable work, not an immediate callback or an always-running JavaScript process. Registration persists until cleared. Scheduling failures are logged without preventing native notification presentation; use regular synchronisation to recover missed work.
