---
title: "Notifications"
description: "Local reminders and routes back into the app."
---

Use `@ink/notifications` to show a notification or schedule a local reminder. Tapping it can open a screen in your app.

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

Request permission through an explicit foreground action where required. Stable IDs update or replace the same notification. `href` names a registered screen; `data` is a string. `useNotificationTap()` exposes the tapped ID and data through its state, and `consume()` clears the tap. Decode data before using it; opening after process death must reconstruct the screen from durable data.

## Scheduling

`schedule({ id, at, title, body, href, data, exact })` stores a native schedule. `cancel(id)` removes it. Scheduling is inexact by default. For an exact reminder, first check `await notifications.canScheduleExact()`. `requestExactPermission()` opens Android's special-access settings; check again after returning before scheduling with `exact: true`. A denied request fails rather than silently becoming inexact. This follows [Android's exact-alarm access model](https://developer.android.com/develop/background-work/services/alarms).

`href` also accepts `{ path: "/departures", params: { stopId: "station-7" } }`. Parameters are JSON values, limited to 8 KiB, and are restored through `useRouteParams()` after a notification tap, including a cold launch. The complete notification request is limited to 12 KiB. `data` remains an independent string payload.

Define whether a reminder follows an absolute instant or local wall time. Reconcile time-zone changes and edited events. Duplicate delivery and a deleted destination should lead to a useful screen rather than an exception.

## Push and privacy

Use [LightOS](light-sdk.md) for host push integration. Validate incoming account and record IDs, update your app’s data, then decide whether to show a notification.

Notification content may be visible outside the app. Messaging apps can offer sender-only or generic previews. Native host policy may restrict presentation or actions, so exposed capabilities determine what the app can request.
