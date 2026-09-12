---
title: "Location"
description: "Read a location or subscribe to updates."
---

Use `@ink/location` for a single position, live updates or background tracking. It does not require Google Play Services.

## Permissions

Request location access before reading a position or starting a watch.

```ts
import { location } from "@ink/location";

const permission = await location.requestPermission();
```

Use `location.getPermission()` to check access without a prompt. Both methods return `granted`, `denied` or `blocked`. See [Request a permission](/permissions-guide) for handling each result.

Both methods accept `"balanced"` (the default) for approximate access or `"high"` for precise access. Match this to the accuracy used by your location request.

For precise access:

```ts
const permission = await location.requestPermission("high");
```

`requestTrackingPermission(accuracy)` requests location and notification access for [background tracking](#background-tracking). It returns the same permission statuses.

## Read a position

`location.current()` returns one position:

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

`selectNearbyPlace` is your app’s function. A location result includes coordinates, a timestamp and accuracy in metres. A cached approximate position may suit a forecast but not navigation.

Pass `signal` to cancel a pending read.

## Watch position changes

`location.watch()` provides positions as they arrive. `updatePosition` handles them in your app:

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

| Option | Meaning | Range |
| --- | --- | --- |
| `interval` | Requested minimum interval in milliseconds. | 1,000–3,600,000 |
| `distance` | Minimum distance in metres. | 0–100,000 |

Updates may arrive less frequently. If your app processes them slowly, it receives the latest position rather than a growing queue. Up to 16 watches can be active.

Break the loop or abort its signal to stop watching. Clean up when leaving the screen. Watches pause when the app enters the background and resume when it returns.

### Availability and accuracy

Balanced requests use Android’s fused location provider when available, otherwise its network provider. High-accuracy requests use GPS and network providers.

Permission uses the LightOS prompt on the phone and Android’s prompt on the emulator. Handle denied permission, disabled location services, timeouts and unavailable providers separately. Offer a manual location choice when appropriate.

## Background tracking

To keep tracking in the background, start tracking from a user action while the app is visible:

```ts
if (await location.requestTrackingPermission() === "granted") {
  await location.startTracking({ interval: 5_000, distance: 10 });
}

const tracking = await location.getTracking();
// { running, fix, error } — fix is the latest recorded position, or null.
```

Stop tracking from a separate action:

```ts
await location.stopTracking();
```

Tracking requires location and notification permission. It shows an ongoing notification with a **Stop** action and continues after leaving the screen. It cannot start while the app is hidden and does not restart after process death or reboot.

`getTracking()` keeps the latest position after tracking stops. Check its timestamp before use. Ink does not save a journey history. Ordinary watches do not automatically become background tracking.

Use a separate provider to search places or convert coordinates into addresses.
