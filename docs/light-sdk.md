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

Ink uses LightOS for camera, microphone, audio files and supported location requests. Photo and video library access uses Android. Without LightOS, requests use Android. Denied or blocked LightOS access does not trigger an Android fallback.

Notification checks and requests use LightOS when the connected Light SDK server is 0.2.4 or later. Older servers, including 0.1.1, use Android. If Ink cannot read the server version, notification access also uses Android.

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
  url: "https://api.example.com/v1/push/subscriptions",
  fetchOnWake: true,
}));
```

LightOS does not deliver push bodies to your app. Treat each push as a wake-up signal: queue events in your subscription server’s inbox, then POST a wake-up to the registered UnifiedPush endpoint. Ink fetches the inbox, applies the events, displays notifications and acknowledges the deliveries.

Call `register.run()` once `push.ready` is true. Set `bearerToken` if your server requires authentication.

Registration sends `PUT {url}/{installationId}` with `{ "endpoint": "…" }`. Your server must return a successful HTTP status and `{ "success": true }`.

### Encrypted notifications

Use encryption for private content. Set the same encryption key on your server and in registration:

```ts
await push.register({
  url: "https://api.example.com/v1/push/subscriptions",
  bearerToken,
  encryptionKey,
  fetchOnWake: true,
});
```

Ink saves the key in secure storage and decrypts notifications on Android, even when the app is closed. The key is not sent to the subscription server. Unregistering removes it.

Encrypt the [notification envelope](#send-a-notification) using the [Ink cipher format](/crypto#server-format). Store this wrapper in the server inbox:

```json
{ "encrypted": "ink1.…" }
```

When encryption is enabled, plaintext notifications are rejected.

### Retrieve notifications after a wake-up

Set `fetchOnWake: true` to enable wake-and-fetch delivery. You can use it with or without `encryptionKey`. Ink also fetches automatically when the push body is empty, `null` or cannot be parsed as an envelope, provided you have an active subscription server registration. Without an inbox, Ink logs the rejected wake-up and drops it.

Use HTTPS for your subscription server. HTTP is allowed for local development at `localhost`, `127.0.0.1` or the emulator host `10.0.2.2`.

Your server must implement these requests at `{url}/{installationId}`:

| Method | Request | Response |
| --- | --- | --- |
| `PUT` | `{ "endpoint": "…" }` | `{ "success": true }` |
| `GET` | No body. Fetch pending deliveries. | `{ "messages": [...] }` |
| `POST` | `{ "ids": ["receipt-42"] }` | `{ "success": true }` |
| `DELETE` | No body. Remove the subscription. | `{ "success": true }` |

Return a successful HTTP status for each request. All four methods use `Authorization: Bearer <bearerToken>` when you provide a token. Ink does not follow redirects.

For a plaintext inbox, each message contains a unique receipt `id` and a version-1 envelope:

```json
{
  "messages": [{
    "id": "receipt-42",
    "version": 1,
    "events": [{
      "operation": "show",
      "id": "message-42",
      "groupKey": "conversation-7",
      "title": "New message",
      "body": "Your message is ready",
      "route": "/inbox"
    }]
  }]
}
```

For an encrypted inbox, return `{ "messages": [{ "encrypted": "ink1.…" }] }`. Ink computes each receipt ID as the lowercase hexadecimal SHA-256 hash of the encrypted string’s UTF-8 bytes. This preserves the encrypted delivery contract; an optional server-supplied `id` is ignored.

Retain deliveries until Ink acknowledges them with `POST`. Remove only acknowledged receipt IDs. Keep event IDs stable when retrying so Ink can remove duplicates. Ink acknowledges after applying the whole inbox; it does not acknowledge a batch that fails validation or decryption. An empty inbox returns `{ "messages": [] }`. Limit each response to 128 messages and 1 MiB.

### Test with the template

From the repository root, start the in-memory development server and forward the phone’s port:

```sh
bun scripts/push-server.ts
# In another terminal:
adb -s LP3LHMA531900140 reverse tcp:18080 tcp:18080
```

Open **Light SDK > Push** in the template and select **Register Push**. The example registers `http://127.0.0.1:18080/v1/push/subscriptions` with `fetchOnWake: true` and no encryption key. The development server supports plaintext inboxes only; use encrypted inboxes for private production content.

List installation IDs, then replace `<installationId>` with your ID to queue an event and wake the phone:

```sh
curl http://127.0.0.1:18080/v1/push/subscriptions
curl -X POST "http://127.0.0.1:18080/v1/push/subscriptions/<installationId>/send" \
  -H 'Content-Type: application/json' \
  -d '{"operation":"show","id":"test-1","groupKey":"test","title":"Test push","body":"Wake-and-fetch works","route":"/modules/light-sdk/push"}'
```

The server queues the event before posting `{}` to the registered LightOS endpoint. It keeps pending events if the wake fails. Use a new event ID for each test. You can also send a complete `{"version":1,"events":[...]}` envelope, including `clear` events. Subscriptions and events are lost when you stop the server.

Select **Unregister** before stopping the server, then remove the port forward:

```sh
adb -s LP3LHMA531900140 reverse --remove tcp:18080
```

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

Queue a JSON envelope in your server inbox before sending the wake-up:

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

A force-stopped app receives no pushes until you open it again. This includes apps stopped with `adb shell am force-stop`. Open the app after installation or a force-stop before testing delivery.

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
