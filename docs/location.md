# Location

`@ink/location` provides foreground, screen-scoped location without bundling a mapping SDK.

```tsx
import { currentLocation, locationPermission } from "@ink/location";

const permission = locationPermission();
const location = currentLocation({
  accuracy: "precise",
  maxAgeMs: 60_000,
  timeoutMs: 15_000,
});
```

`locationPermission()` reads and requests permission through LightOS. Precise location is the default; pass `"approximate"` to request coarse location instead. The compiler contributes the required Android declarations and enables the Light SDK connection automatically.

`currentLocation()` returns a recent GPS, network or passive fix when one satisfies `maxAgeMs`. Otherwise it requests a fresh foreground fix from the available GPS and network providers. Requests are cancelled when the resource reloads, its screen becomes inactive or the app closes.

The ready value contains `latitude`, `longitude`, accuracy in metres, provider and the Unix timestamp in milliseconds. Errors distinguish denied or blocked permission, disabled location, timeout, unavailable providers, invalid native data and unexpected failures.

Continuous tracking, background location, geocoding, routing and mapping are intentionally outside the first version.
