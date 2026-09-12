---
title: "Notifications"
description: "Local reminders and routes back into the app."
---

Use `@ink/notifications` to show a notification or schedule a local reminder. Tapping it can open a screen in your app.

## Permissions

Request notification access when the user enables reminders or another notification feature.

```ts
import { notifications } from "@ink/notifications";

const permission = await notifications.requestPermission();
```

Use `notifications.getPermission()` to check access without a prompt. Both methods return `granted`, `denied` or `blocked`. See [Request a permission](/permissions-guide) for handling each result.

### Exact reminders

Android requires separate access to schedule reminders at an exact time:

```ts
if (!await notifications.canScheduleExact()) {
  await notifications.requestExactPermission();
}
```

`requestExactPermission()` returns after opening settings; it does not wait for a decision. When the user returns to the app, check again before scheduling:

```ts
const allowed = await notifications.canScheduleExact();
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

`useNotificationTap()` provides the tapped ID and data. Call `consume()` to clear the tap. Validate the data and load saved records when a notification starts the app.

## Scheduling

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

Cancel a reminder by ID:

```ts
await notifications.cancel("tea");
```

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

Update reminders when events or time zones change. Handle duplicate taps and records that have since been deleted.

## Push and privacy

Use [LightOS](/light-sdk) for host push integration. Validate incoming account and record IDs, update your app’s data, then decide whether to show a notification.

Notification content can be visible outside the app. Offer limited previews for private messages. LightOS may restrict how notifications and actions appear.
