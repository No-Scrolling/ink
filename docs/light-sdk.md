# LightOS capabilities

`@ink/light-sdk` is an ahead-of-time package. Its TypeScript file is never executed and Ink does not include the full Light SDK client or a JavaScript runtime. Import only the capabilities an application uses; ringtone hand-off and UnifiedPush each contribute their Android source, manifest entries and dependency only when selected by the compiler.

## Dialler and ringtone

`openDialler(phoneNumber)` asks LightOS to open its dialler with the supplied number. It does not place a call.

`ringtoneInstaller()` is a screen-scoped operation controller:

```tsx
const ringtone = ringtoneInstaller();

<Button onPress={() => ringtone.set("./assets/tone.mp3", "ringtone")}>
  Install ringtone
</Button>
```

The source must be a local string literal. Ink bundles it, stages a private read-only copy and gives only the system LightOS process access through a conditional content provider. `kind` is `"ringtone"`, `"notification"` or `"alarm"` and defaults to `"ringtone"`. The controller moves through `idle`, `installing`, `installed` or `error`; the error branch exposes the same `error.kind`, `error.message` and `error.retryable` shape as every Ink module. Ink deliberately has no general shared-files interface.

## UnifiedPush

`lightPush()` is application-scoped and may be declared only once. Notification permission remains explicit through `notificationPermission()` from `@ink/notifications`.

```tsx
const permission = notificationPermission();
const push = lightPush();

<Button onPress={() => permission.request()}>Allow notifications</Button>
<Button onPress={() => push.register("https://example.com/push/subscriptions")}>
  Register push
</Button>
```

Registration creates a durable installation ID, registers the `light-push` instance with the LightOS UnifiedPush distributor and sends this request to the application's subscription service:

```http
PUT /push/subscriptions/<installation-id>
Content-Type: application/json
Authorization: Bearer <optional-token>

{"endpoint":"<unified-push-endpoint>"}
```

The service must treat `PUT` and `DELETE` as idempotent. `unregister()` unregisters the connector and issues `DELETE` to the same installation URL. `retry()` repeats a failed connector or endpoint synchronisation. Production subscription URLs must use HTTPS; HTTP is accepted only for emulator loopback hosts.

The push body is UTF-8 JSON:

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

`show` requires `id`, `groupKey`, `title` and `body`; `route` and `sentAtMs` are optional. `clear` requires only a new event `id` and the `groupKey` to remove. Event IDs deduplicate retries. A newer `show` with the same group key replaces the projected inbox record and displayed notification, matching a conversation or room. `dismiss(groupKey)` removes one local group and `clear()` removes the whole local inbox without unregistering.

Ink accepts at most 4,096 bytes and 16 events per envelope, retains 64 group records and the latest 512 event IDs, and bounds every string before persistence. It writes the inbox before presenting a notification, so delivery survives a killed UI process. A notification tap removes that group, records `openedKey`, starts the activity through an Android activity pending intent and navigates through Ink's validated route seam. `messages` projects the remaining generic records for TSX rendering; application-specific payloads remain outside Ink's interface.

The connector endpoint, desired registration, optional bearer token, inbox and deduplication window live in an app-private atomic file. A process restart resumes an interrupted registration or endpoint synchronisation; retry after a reported error is explicit. Notification display depends on Android notification permission, but payload persistence does not.

## Host-owned preferences and keys

Ink refreshes LightOS haptic, emoji and key-animation preferences on connection and application resume, then applies the supported values to its small native keyboard. Voice entry and swipe typing are not packaged. Recognised LP3 device-key events are forwarded to LightOS with the current activity as the relaunch component; Back and Home retain Android system behaviour. These are host concerns and add no author-facing API.
