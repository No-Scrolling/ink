---
title: "Notifications"
description: "Local reminders and routes back into the app."
tag: "Design specification"
---

`@ink/notifications` creates native notifications and schedules supported local reminders. Use them for an actionable event, such as a download finishing or a saved departure approaching.

```ts
import { notifications } from "@ink/notifications";

await notifications.show({
  id: `download:${downloadId}`,
  title: "Download complete",
  body: "Your episode is ready offline",
  route: { path: "/downloads", params: { downloadId } },
});
```

Request permission through an explicit foreground action where required. Stable IDs update or replace the same notification. A route contains small validated JSON values; opening it after process death must reconstruct the screen from durable data.

## Scheduling

`schedule({ id, at, title, body, route })` stores a native schedule. `cancel(id)` removes it. Scheduling is inexact by default and must be presented accordingly. Exact alarms require a separately supported Android capability and permission; ordinary scheduling does not promise exact delivery.

Define whether a reminder follows an absolute instant or local wall time. Reconcile time-zone changes and edited events. Duplicate delivery and a deleted destination should lead to a useful screen rather than an exception.

## Push and privacy

Push transport is a provider/host integration, described under [LightOS](light-sdk.md). Receiving a payload, reconciling domain data and deciding to display a notification are separate operations. Validate account and record IDs; never execute commands just because a payload names them.

Notification content may be visible outside the app. Messaging apps can offer sender-only or generic previews. Native host policy may restrict presentation or actions, so exposed capabilities determine what the app can request.
