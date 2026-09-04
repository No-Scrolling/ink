---
title: "Location"
description: "One-off fixes and visibility-scoped location updates."
tag: "Design specification"
---

`@ink/location` provides native location without requiring a Google Play Services provider. Use a one-off fix for weather or nearby departures, and a visible watch when movement matters.

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

The final operation belongs to the app. A fix includes timestamp, accuracy in metres and coordinates. Check age and accuracy before using it; a cached coarse fix may be sufficient for a forecast but unsuitable for turn guidance.

`location.current({ signal })` accepts cancellation. `location.watch(options, listener)` returns an unsubscribe function and should be acquired inside `useVisibleEffect`. Choose a minimum distance and update interval appropriate to the task. The OS may deliver more slowly than requested.

Permissions are requested from an explicit user action. Denied permission, disabled location services, timeout and unavailable providers are separate states. A manual saved-place choice is a useful fallback for weather and transit apps.

Background location requires a separately declared capability and a native lifecycle appropriate to that feature. A foreground watch does not become a tracking service when its screen disappears. Geocoding and place search belong to a chosen provider, not the raw location API.
