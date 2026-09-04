---
title: "Maps"
description: "Native map views with explicit providers and offline regions."
tag: "Design specification"
---

`@ink/maps` provides the native map surface; a provider package supplies its map source, attribution and supported capabilities. Choose and configure that provider explicitly.

```tsx
import { Screen } from "ink";
import { MapView } from "@ink/maps";
import { mapSource } from "./map-provider";

export function Nearby({ latitude, longitude }: {
  latitude: number; longitude: number;
}) {
  return (
    <Screen title="Nearby">
      <MapView source={mapSource} centre={{ latitude, longitude }} zoom={14} />
    </Screen>
  );
}
```

`mapSource` is app configuration for an installed provider, not an implicit free tile service. Package size, rendering features, access tokens, attribution and offline rights depend on that provider.

The native view owns panning, zooming, tile decoding and labels. Initial centre/zoom seed the view; use an explicit camera command for subsequent movements. Observe settled camera changes when useful, rather than sending every gesture frame through JavaScript.

Markers use stable IDs, coordinates and compact metadata. Use native clustering for dense collections. A selected marker can open an Ink detail screen by record ID; do not recreate the map whenever a label changes.

Offline regions are explicit native download jobs with size estimates, progress and removal. A transient tile cache is not an offline guarantee. Register only supported provider capabilities, and retain attribution when displaying cached content.

Use [Location](location.md) for the device position. Routing, geocoding and traffic are separate provider services. A simple departure board may be clearer and smaller without a map.
