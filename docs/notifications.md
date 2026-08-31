# Notifications

`@ink/notifications` provides notification permission, durable local reminders and notification-tap events.

## Permission

`notificationPermission()` returns `"granted"`, `"denied"`, `"blocked"`, or `"unknown"` when ready. Call `request()` from a user action to open the Android permission prompt.

```tsx
import { notificationPermission } from "@ink/notifications";
import { Button } from "ink";

const permission = notificationPermission();

<Button onPress={() => permission.request()}>Allow notifications</Button>
```

Ink adds `POST_NOTIFICATIONS` when the app declares this resource or LightOS push.

## Schedule a reminder

```tsx
import { localNotifications } from "@ink/notifications";
import { Button } from "ink";

const notifications = localNotifications();

<Button onPress={() => notifications.schedule({
  id: "daily-review",
  title: "Daily review",
  body: "Take a moment to review today.",
  href: "/review",
  data: "daily",
  delayMs: 60_000,
})}>
  Schedule review
</Button>
```

A notification requires exactly one schedule field:

- `delayMs` for a delay from now;
- `triggerAtMs` for a Unix timestamp in milliseconds.

Scheduling an existing ID replaces its pending or displayed notification. `cancel(id)` removes both. IDs must match `[A-Za-z0-9][A-Za-z0-9._-]{0,63}`.

Android may defer delivery around power and system policy. Reminders survive process death and device restarts, but they are not exact alarms. An app can retain up to 128 pending notifications, displayed notifications, and unconsumed taps in total.

## Handle notification taps

`notificationTap()` exposes the oldest unconsumed tap:

```tsx
import { notificationTap } from "@ink/notifications";
import { Text, match } from "ink";

const tap = notificationTap();

{match(tap, {
  empty: () => <Text>No notification tap</Text>,
  ready: (result) => <Text>{result.value.data}</Text>,
})}
```

Tap events are stored before the activity opens and survive process death. The event remains `ready` until `consume()` acknowledges it; the next queued event then becomes visible.

If the notification includes `href`, Ink also opens that validated route. Route navigation does not depend on consuming the event.

## Errors and packaging

Scheduling and cancellation expose `idle` or `error`. An error includes the operation, notification ID, and a structured Ink error.

The module adds its Android permission, alarm receiver, tap receiver, and durable storage only when notifications are used. LightOS UnifiedPush uses the same notification presentation and route handling. Read [LightOS](light-sdk.md) for push registration and payloads.
