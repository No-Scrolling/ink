---
title: "Notifications"
description: "Local reminders and routes back into the app."
---

Use `@ink/notifications` to show a notification or schedule a local reminder. Tapping it can open a screen in your app.

## Permissions

| Permission | Required for |
| --- | --- |
| `notifications` | Showing notifications and scheduling reminders. |
| [Exact alarm access](#exact-reminders) | Reminders with `exact: true`, in addition to notification access. |

```ts
import { lightos } from "@ink/lightos";

const permission = await lightos.requestPermission("notifications");
```

Use `lightos.getPermission("notifications")` to check without a prompt. See [Request a permission](/permissions-guide) for the returned statuses.

### Exact reminders

Android requires separate access to schedule reminders at an exact time:

```ts
if (!await lightos.canScheduleExact()) {
  await lightos.requestExactPermission();
}
```

`requestExactPermission()` returns after opening settings; it does not wait for a decision. When the user returns to the app, check again before scheduling:

```ts
const allowed = await lightos.canScheduleExact();
```

Only schedule with `exact: true` when `allowed` is true.

## Show a notification

```ts
import { notifications } from "@ink/notifications";

await notifications.show({
  id: `download-${downloadId}`,
  title: "Download complete",
  body: "Your episode is ready offline",
  href: "/downloads",
  data: JSON.stringify({ downloadId }),
});
```

Reuse an `id` to update or replace a notification. `href` names a registered screen; `data` is a string.

## Schedule a reminder

Schedule a reminder for ten minutes from now:

```ts
import { notifications } from "@ink/notifications";

await notifications.schedule({
  id: "tea",
  at: Date.now() + 10 * 60 * 1_000,
  title: "Tea is ready",
  body: "Your timer has finished",
});
```

`at` is a timestamp in milliseconds. Delivery time is approximate by default. `exact: true` requires exact-alarm access; without it, scheduling fails.

### Cancel a reminder

```ts
await notifications.cancel("tea");
```

## Handle notification taps

`useNotificationTap()` provides the tapped ID and data. Call `consume()` to clear the tap. Validate the data and load saved records when a notification starts the app.

### Pass screen parameters

For an app with a `/departures` screen:

```ts
await notifications.show({
  id: "departure-station-7",
  title: "Bus arriving",
  body: "Your bus arrives in five minutes",
  href: { path: "/departures", params: { stopId: "station-7" } },
});
```

Read parameters with `useRouteParams()` after a tap, including when the app starts from closed.

Parameters must be JSON values totalling at most 8 KiB. The complete notification request is limited to 12 KiB. `data` is a separate string payload.

Handle duplicate taps and records that have since been deleted. Update reminders when events or time zones change.

## Push notifications

Use [LightOS](/light-sdk) for host push integration. Validate incoming account and record IDs, update your app’s data, then decide whether to show a notification.

### Private content

Notification content can be visible outside the app. Offer limited previews for private messages. LightOS may restrict how notifications and actions appear.
