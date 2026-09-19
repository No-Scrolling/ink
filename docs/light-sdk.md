---
title: "LightOS"
description: "Read LightOS preferences and use system services."
---

Use `@ink/lightos` to read LightOS preferences, request permissions or open the dialler. Ringtones and push notifications use separate imports.

Importing `@ink/lightos` includes the integration and registers the app’s required services. Available features depend on the installed LightOS version.

## Permissions

| Permission | Access |
| --- | --- |
| `camera` | Camera. |
| `microphone` | Microphone. |
| `location-approximate` | Approximate location. |
| `location-precise` | Precise location. |
| `notifications` | Notifications. |
| `audio-files` | Audio files in the device media library. |
| `photos` | Photo library. |
| `videos` | Video library. |
| `photos-and-videos` | Photos and videos. |

```ts
import { lightos } from "@ink/lightos";

const permission = await lightos.requestPermission("microphone");
```

Use `lightos.getPermission(name)` to check without a prompt. Both methods return `granted`, `denied` or `blocked`. See [Request a permission](/permissions-guide) for handling each result.

Ink uses LightOS for camera, microphone, audio files and supported location requests. Other permissions use Android. Without LightOS, requests use Android. Denied or blocked LightOS access does not trigger an Android fallback.

Exact reminders use `lightos.canScheduleExact()` and `lightos.requestExactPermission()`. See [Notifications](/notifications#exact-reminders).

## Read preferences

Read haptic and keyboard settings:

```ts
import { lightos } from "@ink/lightos";

const preferences = await lightos.getPreferences(); // hapticsEnabled
const keyboard = await lightos.getKeyboardOptions();
```

These methods return current values, not subscriptions. Read them again when returning to the app.

| Keyboard option | Value |
| --- | --- |
| `emojis` | Configured emoji string, or `null` when unavailable. |
| `displayVoice` | Whether to show voice input. |
| `enableKeyAnimation` | Whether key animations are enabled. |
| `swipeEnabled` | Swipe typing preference, or `null` on older hosts. |

## Open the dialler

Pass the user-selected phone number:

```ts
import { lightos } from "@ink/lightos";

await lightos.openDialler({ phoneNumber });
```

A successful response means LightOS accepted the request. Services can report unavailable, denied or blocked-by-host outcomes. Android permission does not override LightOS restrictions.

## Check the LightOS version

```ts
import { lightos } from "@ink/lightos";

const version = await lightos.getVersion();
```

LightOS does not expose account details or a complete list of supported features. Check individual permissions and handle unavailable operations instead of relying on the version number.

## Push notifications

`@ink/lightos/push` uses the configured LightOS UnifiedPush distributor to deliver notifications. `usePush()` provides registration state, a saved inbox and commands to manage both.

### Register with your server

Inside your component:

```ts
import { useAction } from "ink";
import { usePush } from "@ink/lightos/push";

const push = usePush();
const register = useAction(() => push.register({
  url: "https://api.example.com/push",
}));
```

Call `register.run()` once `push.ready` is true. Set `bearerToken` if your server requires authentication.

Registration sends `PUT {url}/{installationId}` with `{ "endpoint": "…" }`. Your server must return a successful HTTP status and `{ "success": true }`.

### Encrypted notifications

Set the same encryption key on your server and in registration:

```ts
await push.register({
  url: "https://api.example.com/push",
  bearerToken,
  encryptionKey,
});
```

Ink saves the key in secure storage and decrypts notifications on Android, even when the app is closed. The key is not sent to the subscription server. Unregistering removes it.

Encrypt the [notification payload](#send-a-notification) using the [Ink cipher format](/crypto#server-format). Send this wrapper with `Content-Type: application/json`:

```json
{ "encrypted": "ink1.…" }
```

When encryption is enabled, plaintext notifications are rejected.

### Foreground notifications

Use the notification's `groupKey` to identify the content on screen. For a conversation, this can be its ID:

```ts
import { usePush } from "@ink/lightos/push";

const push = usePush({ foregroundGroup: conversationId });
```

Ink clears existing notifications for this group and suppresses new ones while the app has focus. Pass `null` when no specific group is on screen, such as when returning to the inbox.

When a new message arrives through your live connection, trigger a haptic for the visible group:

```ts
await push.notifyForeground(conversationId);
```

This does nothing if another group is visible or the app is in the background.

### Unregister

```ts
await push.unregister();
```

This sends `DELETE` to the same URL.

`retry()` resumes failed registration or pending unsubscription. Both survive app restarts.

### Send a notification

Your server sends a JSON payload:

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

New messages replace notifications in the same group. Ink applies the whole batch before showing notifications. Tapping one removes its inbox entry and opens its optional route.

The optional top-level `expiresAtMs` sets an expiry as a Unix timestamp in milliseconds. Ink ignores expired payloads and duplicate event IDs. Without it, notifications do not expire.

Use `operation: "clear"` with an `id` and `groupKey` to remove a notification. In the app, use `dismiss` and `clear` to manage the inbox.

### Delivery limits

Notification JSON is limited to 4 KiB and 16 events, before encryption. Ink keeps 64 inbox entries and 512 recent event IDs to recognise duplicates.

Ink saves and displays notifications natively. A [background task](#background-push-tasks) can process deliveries without opening the app. Push can be delayed, duplicated or unavailable, so also sync data independently.

## Background push tasks

### Define the handler

Add a task to your [worker file](/background):

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

`reconcileInbox` is your app’s function. It receives new messages and cancelled group keys after Ink removes duplicates.

### Register the handler

```ts
import { setPushTask } from "@ink/lightos/push";
import { processPush } from "./workers";

await setPushTask(processPush);
// Remove the handler when signing out:
await setPushTask(null);
```

Registration remains until cleared. Android may delay the task; each run has a two-minute limit and a cancellation signal.

Scheduling failures are logged but do not prevent notifications appearing. Regular syncing should recover missed work.

## Device setup

Use `ink dev --device <serial>` when switching devices. An APK built for the emulator’s LightOS host cannot connect to the LP3 host. `ink info` reports integration requirements. Missing host services report unavailable errors.

The template’s push example requires a local subscription server and an ADB reverse connection on port 18080.

Ink routes hardware keys to focus, navigation and active media. Avoid competing listeners for the same keys.

Contact Light for distribution approval and access to restricted services. Enabling the integration does not grant either.
