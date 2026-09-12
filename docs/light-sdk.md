---
title: "LightOS"
description: "Read LightOS preferences and use system services."
---

Use `@ink/lightos` to read LightOS preferences, request permissions or open the dialler. Ringtones and push notifications use separate imports.

Enable host integration in the app configuration:

```toml
[lightos]
enabled = true
```

Ink registers the app and its required services. Available features depend on the installed LightOS version.

## Permissions

Use these methods when a feature requires permission from LightOS:

```ts
import { lightos } from "@ink/lightos";

const permission = await lightos.requestPermission("microphone");
```

| Permission | Access |
| --- | --- |
| `camera` | Camera. |
| `microphone` | Microphone. |
| `location-approximate` | Approximate location. |
| `location-precise` | Precise location. |

Use `lightos.getPermission(name)` to check without a prompt. Both methods return `granted`, `denied` or `blocked`. See [Request a permission](/permissions-guide) for handling each result.

These methods require LightOS. To support devices without it, use the feature module’s permission methods, such as [Camera](/camera#permissions) or [Location](/location#permissions).

## Preferences and system actions

Read preferences or open the dialler with a user-selected phone number:

```ts
import { lightos } from "@ink/lightos";

const version = await lightos.getVersion();
const preferences = await lightos.getPreferences(); // hapticsEnabled
const keyboard = await lightos.getKeyboardOptions();
await lightos.openDialler({ phoneNumber });
```

Services can report unavailable, denied or blocked-by-host outcomes. Android permission does not override LightOS restrictions. A successful response means LightOS accepted the request; check the visible or audible result on your target phone.

Ink routes hardware keys to focus, navigation and active media. Avoid adding listeners that compete for the same keys.

### Refresh preferences

`getPreferences()` and `getKeyboardOptions()` return current values, not subscriptions. Read them again when returning to the app. Keyboard options include `emojis`, `displayVoice`, `enableKeyAnimation` and nullable `swipeEnabled`, which older hosts omit.

LightOS does not expose account details or a complete list of supported features. Check individual permissions and handle unavailable operations instead of relying on the version number.

## Push notifications

`usePush()` from `@ink/lightos/push` provides registration state, a saved inbox and `register`, `retry`, `unregister`, `dismiss` and `clear` commands. It uses the configured LightOS UnifiedPush distributor, the service that delivers notifications to the phone.

Inside your component, create the push controller and a registration action:

```ts
import { useAction } from "ink";
import { usePush } from "@ink/lightos/push";

const push = usePush();
const register = useAction(() => push.register("https://api.example.com/push"));
```

Use your subscription server’s URL. Call `register.run()` from a button once `push.ready` is true. Pass a bearer token as the second argument if your server requires authentication.

Registration sends `PUT {subscriptionBaseUrl}/{installationId}` with `{ "endpoint": "…" }`. Your server must return a successful HTTP status.

Unregister from a separate action:

```ts
await push.unregister();
```

This sends `DELETE` to the same URL.

Send notifications in this format:

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

Use `operation: "clear"` with an `id` and `groupKey` to remove a notification. New messages replace notifications in the same group. Ink applies the whole batch before showing the remaining notifications. Tapping one consumes its inbox entry and opens its optional route.

Each payload allows up to 4 KiB and 16 events. Ink keeps 64 inbox entries and 512 recent event IDs to recognise duplicates.

`retry()` resumes failed registration or pending unsubscription. Both survive app restarts.

Ink saves and displays notifications natively. A [background task](#background-push-tasks) can process deliveries without opening the app. Push can be delayed, duplicated or unavailable, so also sync data independently.

## Background push tasks

Define a handler in your [worker file](/background) and register it from the UI with `setPushTask(task)`.

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

`reconcileInbox` is your app’s function. It receives new messages and cancelled group keys after Ink removes duplicates.

Android schedules the task in a new worker runtime with a two-minute limit, cancellation signal and worker APIs. The task may be delayed. Registration remains until cleared.

Scheduling failures are logged but do not prevent notifications appearing. Regular syncing should recover missed work.

## Device setup

Use `ink dev --device <serial>` when switching devices. An APK built for the emulator’s LightOS host cannot connect to the LP3 host. `ink info` reports integration requirements. Missing host services report unavailable errors.

The template’s push example requires a local subscription server and an ADB reverse connection on port 18080. Dialler presentation has been checked on the LP3; audible ringtones and production push still need device checks.

Contact Light for distribution approval and access to restricted services. Enabling the integration does not grant either.
