# Notifications

Install `@ink/notifications` and import `notificationPermission`, `localNotifications` or `notificationTap` as needed.

`notificationPermission()` exposes the Android notification permission as a resource with `granted`, `denied` and `blocked` values. Calling `request()` opens the platform prompt. Ink contributes `POST_NOTIFICATIONS` only when this resource is declared.

`localNotifications()` schedules and cancels generic reminders:

```tsx
const notifications = localNotifications();

notifications.schedule({
  id: "daily-review",
  title: "Daily review",
  body: "Take a moment to review today.",
  href: "/review",
  data: "daily",
  delayMs: 60_000,
});
```

A notification requires exactly one of `delayMs` or `triggerAtMs`. Scheduling an existing ID atomically replaces its pending alarm or displayed notification. `cancel(id)` removes both. Android uses one high-importance **Reminders** channel and `setAndAllowWhileIdle`, so delivery is durable but not exact. Reboots restore future alarms. Ink retains at most 128 pending notifications, displayed notifications and unconsumed taps together.

`notificationTap()` reads the oldest durable tap event. Taps are written before the activity opens and survive process death. The event remains `ready` until `consume()` acknowledges it; the next queued tap then becomes visible. If a notification includes `href`, Ink also navigates to that route independently of event acknowledgement.

Notification IDs match `[A-Za-z0-9][A-Za-z0-9._-]{0,63}`. Titles, bodies, routes and data are bounded before storage. The Android presenter and keyed event queue are internal shared seams so remote Light push can use the same replacement and delivery semantics later.

The AlarmManager, NotificationManager, receiver and durable-store implementation is conditionally packaged only when `@ink/notifications` is imported. It does not use AndroidX NotificationCompat or WorkManager.
