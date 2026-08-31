---
title: "Location"
description: "Request permission and read a foreground location fix."
---

`@ink/location` provides one foreground location fix without bundling maps, routing, or geocoding.

## Permission

`locationPermission()` reads and requests permission through LightOS. Precise permission is the default; pass `"approximate"` to request coarse location.

```tsx
import { locationPermission } from "@ink/location";
import { Button } from "ink";

const permission = locationPermission("precise");

<Button onPress={() => permission.request()}>Allow location</Button>
```

Creating the permission resource does not open a prompt. Ink adds the Android declarations and LightOS connection when the module is imported.

## Read a location

```tsx
import { currentLocation } from "@ink/location";
import { Text, match } from "ink";

const location = currentLocation({
  accuracy: "precise",
  maxAgeMs: 60_000,
  timeoutMs: 15_000,
});

{match(location, {
  loading: () => <Text>Finding location</Text>,
  ready: (result) => (
    <Text>{result.value.latitude}, {result.value.longitude}</Text>
  ),
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

`currentLocation()` returns a recent GPS, network, or passive fix when it satisfies `maxAgeMs`. Otherwise it requests a fresh foreground fix from the available providers.

| Option | Values | Default |
| --- | --- | --- |
| `accuracy` | `"approximate"`, `"precise"` | `"precise"` |
| `maxAgeMs` | 0 to 3,600,000 milliseconds | 60,000 milliseconds |
| `timeoutMs` | 1,000 to 120,000 milliseconds | 15,000 milliseconds |

The ready value contains:

- `latitude` and `longitude`;
- `accuracy` in metres;
- `provider` as `"gps"`, `"network"`, or `"passive"`;
- `timestamp` as Unix time in milliseconds.

## Lifecycle and errors

Location work is screen-scoped. A request is cancelled when it reloads, its screen leaves, or the app closes.

Errors distinguish denied or blocked permission, disabled location, timeout, unavailable providers, invalid native data, and unexpected failures.

The module does not provide continuous tracking, background location, geocoding, routing, or maps.
