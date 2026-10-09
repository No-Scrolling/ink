---
title: "Location"
description: "Read a location or subscribe to updates."
---

Use `@ink/location` for a single position, live updates or background tracking. It does not require Google Play Services.

## Permissions

| Permission | Required for |
| --- | --- |
| `location-approximate` | Positions and watches with `accuracy: "balanced"` (the default). |
| `location-precise` | Positions and watches with `accuracy: "high"`. |

```ts
import { lightos } from "@ink/lightos";

const permission = await lightos.requestPermission("location-approximate");
```

Use `lightos.getPermission("location-approximate")` to check without a prompt. See [Request a permission](/permissions-guide) for the returned statuses.

For precise access:

```ts
const permission = await lightos.requestPermission("location-precise");
```

## Use the LightOS location

`location.default()` reads the location saved in the Light dashboard. It returns `{ latitude, longitude }`, or `null` when no location is set. It does not turn on GPS or fall back to the current position.

```ts
import { lightos } from "@ink/lightos";
import { location } from "@ink/location";

if (await lightos.requestPermission("location-approximate") === "granted") {
  const saved = await location.default();
}
```

This requires Light SDK 0.1.2 or newer. On older hosts, Ink rejects with `NativeError.kind === "unsupported"` without requesting the location. Permission denial and an unavailable host also reject the call; they do not return `null`.

## Read a position

`location.current()` returns one position:

```ts
import { lightos } from "@ink/lightos";
import { location } from "@ink/location";

const permission = await lightos.requestPermission("location-approximate");
if (permission === "granted") {
  const fix = await location.current({
    accuracy: "balanced",
    maximumAge: 300_000,
    timeout: 15_000,
  });
  await selectNearbyPlace(fix.latitude, fix.longitude);
}
```

`selectNearbyPlace` is your app’s function. The result includes coordinates, a timestamp and accuracy in metres.

### Cached positions and cancellation

`maximumAge` is the oldest cached position to accept, in milliseconds. A cached approximate position may suit a forecast but not navigation.

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

Updates may arrive less frequently. Slow consumers receive the latest position. Up to 16 watches can be active.

### Stop watching

Break the loop or abort its signal to stop watching. Clean up when leaving the screen. Watches pause when the app enters the background and resume when it returns.

## Availability and accuracy

Balanced requests use Android’s fused location provider when available, otherwise its network provider. High-accuracy requests use GPS and network providers.

Handle denied permission, disabled location services, timeouts and unavailable providers separately. A manual location choice can provide a fallback.

## Background tracking

Background tracking also requires notification permission because it displays an ongoing notification.

### Start tracking

Start from a user action while the app is visible:

```ts
import { lightos } from "@ink/lightos";
import { location } from "@ink/location";

if (await lightos.requestPermission("location-approximate") === "granted"
  && await lightos.requestPermission("notifications") === "granted") {
  await location.startTracking({ interval: 5_000, distance: 10 });
}

const tracking = await location.getTracking();
// { running, fix, error } — fix is the latest recorded position, or null.
```

### Stop tracking

```ts
await location.stopTracking();
```

The notification also includes a **Stop** action.

### Tracking state

Tracking continues after leaving the screen. It cannot start while the app is hidden and does not restart after process death or reboot.

`getTracking()` keeps the latest position after tracking stops. Check its timestamp before use. Ink does not save a journey history or turn ordinary watches into background tracking.

## Search for places

Use a separate provider to search places or convert coordinates into addresses.
