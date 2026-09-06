---
title: "Location"
description: "One-off fixes and visibility-scoped location updates."
---

Use `@ink/location` for a single position, live updates or background tracking. It does not require Google Play Services.

Request a single position for weather or nearby departures:

```ts
import { location } from "@ink/location";

const permission = await location.requestPermission();
if (permission === "granted") {
  const fix = await location.current({
    accuracy: "balanced",
    maximumAge: 300_000,
    timeout: 15_000,
  });
  await selectNearbyPlace(fix.latitude, fix.longitude);
}
```

`selectNearbyPlace` is your app’s function. A fix includes timestamp, accuracy in metres and coordinates. Check age and accuracy before using it; a cached coarse fix may be sufficient for a forecast but unsuitable for turn guidance.

`location.current({ signal })` accepts cancellation. A watch uses a native listener and yields fresh fixes:

```ts
const controller = new AbortController();
for await (const fix of location.watch({
  accuracy: "balanced",
  interval: 5_000,
  distance: 10,
  signal: controller.signal,
})) {
  updatePosition(fix);
}
```

`interval` is the requested minimum interval in milliseconds (1,000–3,600,000); `distance` is the minimum distance in metres (0–100,000). The provider may deliver less frequently. Breaking the loop releases the listener. Abort the controller when leaving the screen or stopping a pending read. Ordinary watches pause when the app loses the foreground and resume when it returns; they do not create a background service. A slow consumer receives the latest fix, without an unbounded queue. Up to 16 watches can be active.

Balanced requests prefer Android's built-in fused provider when available and otherwise use its network provider. This does not require Google Play Services. High-accuracy requests retain the GPS/network path.

Permissions are requested from an explicit user action. Denied permission, disabled location services, timeout and unavailable providers are separate states. A manual saved-place choice is a useful fallback for weather and transit apps.

To continue tracking while the app is in the background, start the native tracking service from an explicit action while the app is visible:

```ts
if (await location.requestTrackingPermission() === "granted") {
  await location.startTracking({ interval: 5_000, distance: 10 });
}

const tracking = await location.getTracking();
// { running, fix, error } — fix is the latest recorded position, or null.
await location.stopTracking();
```

Tracking requires location and notification permission. It presents an ongoing notification with a Stop action and continues independently of the screen. It does not automatically restart after process death or reboot, or start while the app is hidden. `getTracking()` retains the last recorded fix after stopping; check its timestamp before using it. Tracking stores one latest fix, not a journey history. A screen watch does not turn into a tracking service when its screen disappears.

Geocoding and place search belong to a chosen provider, not the raw location API.
