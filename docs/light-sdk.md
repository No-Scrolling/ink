---
title: "LightOS"
description: "Connect Ink apps to LightOS services and system integrations."
---

`@ink/light-sdk` connects an Ink app to LightOS. It provides service status, LightOS permissions, the dialler, ringtone installation, UnifiedPush, device-key forwarding, and host keyboard preferences.

Ink targets Light SDK `0.1.1`.

## Enable host integration

Add a side-effect import in `App.tsx` when the app needs automatic LightOS preferences or hardware-key forwarding:

```tsx
import "@ink/light-sdk";
```

On a Light Phone III, Ink connects to `com.lightos`. On an Android emulator, `ink dev` uses the official Light SDK emulator service.

## Check the connection

`lightSdkVersion()` reports the connected service version as an async resource.

```tsx
import { lightSdkVersion } from "@ink/light-sdk";
import { Text, match } from "ink";

const version = lightSdkVersion();

{match(version, {
  loading: () => <Text>Connecting to LightOS</Text>,
  ready: (result) => <Text>Light SDK {result.value}</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

Call `reload()` to reconnect after an error.

## Request a LightOS permission

`lightSdkPermission("camera")` reads and requests the LightOS camera permission. The ready value is `"granted"`, `"denied"`, `"blocked"`, or `"unknown"`.

```tsx
import { lightSdkPermission } from "@ink/light-sdk";
import { Button } from "ink";

const camera = lightSdkPermission("camera");

<Button onPress={() => camera.request()}>Allow camera</Button>
```

Creating a permission resource does not open a prompt. Call `request()` from a user action. Higher-level modules such as `@ink/camera` and `@ink/location` use the relevant LightOS permission flow for you.

## Open the dialler

`openDialler(phoneNumber)` opens the LightOS dialler with a number filled in. It does not place the call.

```tsx
import { openDialler } from "@ink/light-sdk";
import { Button } from "ink";

<Button onPress={() => openDialler("+15551234567")}>
  Open dialler
</Button>
```

## Install a ringtone

Use `ringtoneInstaller()` to give a bundled audio file to LightOS.

```tsx
import { ringtoneInstaller } from "@ink/light-sdk";
import { Button, Text, match } from "ink";

const ringtone = ringtoneInstaller();

{match(ringtone, {
  idle: () => <Text>Choose a ringtone</Text>,
  installing: () => <Text>Installing ringtone</Text>,
  installed: () => <Text>Ringtone installed</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
<Button onPress={() => ringtone.set("./assets/tone.mp3", "ringtone")}>
  Install ringtone
</Button>
```

The source must be a bundled string literal. The kind can be `"ringtone"`, `"notification"`, or `"alarm"`; the default is `"ringtone"`.

## Register for UnifiedPush

`lightPush()` owns one app-wide UnifiedPush registration and a durable generic message inbox. Declare it once in the app.

```tsx
import { lightPush } from "@ink/light-sdk";
import { Button } from "ink";

const push = lightPush();

<Button onPress={() => push.register(
  "https://example.com/push/subscriptions",
)}>
  Enable push
</Button>
```

To register for push, Ink performs these actions:

1. Creates a durable installation ID.
2. Registers the `light-push` instance with the LightOS UnifiedPush distributor.
3. Sends the endpoint to your subscription service.

Ink sends this request:

```http
PUT /push/subscriptions/<installation-id>
Content-Type: application/json
Authorization: Bearer <optional-token>

{"endpoint":"<unified-push-endpoint>"}
```

Your service must treat `PUT` and `DELETE` as idempotent. `unregister()` removes the connector and sends `DELETE` to the same installation URL. `retry()` repeats a failed registration or endpoint synchronisation.

Production subscription URLs must use HTTPS. Emulator loopback URLs can use HTTP.

Pass an optional bearer token as the second argument to `register()`. `push.status` is `idle`, `registering`, `synchronising`, `ready`, or `error`.

## Send a push payload

Send UTF-8 JSON with version `1` and up to 16 events:

```json
{
  "version": 1,
  "events": [
    {
      "operation": "show",
      "id": "event-42",
      "groupKey": "room-7",
      "title": "Alex",
      "body": "Are you free?",
      "route": "/messages",
      "sentAtMs": 1788133300000
    }
  ]
}
```

A `show` event requires `id`, `groupKey`, `title`, and `body`. `route` and `sentAtMs` are optional. A `clear` event requires a new event `id` and the `groupKey` to remove.

Event IDs deduplicate retries. A newer `show` with the same group key replaces the existing inbox record and displayed notification. This model suits conversations, rooms, or any other keyed feed.

Payloads are limited to 4,096 bytes. Ink retains up to 64 group records and 512 recent event IDs.

## Read and clear push messages

`push.messages` contains the current inbox. Use:

- `dismiss(groupKey)` to remove one group;
- `clear()` to remove every local group without unregistering;
- `push.openedKey` to identify the group opened from a notification tap.

When an event includes `route`, tapping its notification opens that validated Ink route. Notification display requires Android notification permission from `@ink/notifications`; payload storage does not.

## Automatic host behaviour

When host integration is enabled, Ink applies LightOS haptic, emoji, and keyboard-animation preferences to the Ink keyboard. It also forwards recognised LP3 device keys to LightOS. Android continues to own the Back and Home keys.

Voice input and swipe typing are not supported.

## Packaging

Ink packages LightOS capabilities independently. Ringtone hand-off and UnifiedPush add their Android components only when the app uses those APIs. The module does not add Compose or the full Light SDK client to the APK.
