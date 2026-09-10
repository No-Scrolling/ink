---
title: "Maps"
description: "An optional MapLibre map with markers and native gestures."
---

`@ink/maps` displays a MapLibre map with gestures, tiles and markers. Supply coordinates and callbacks. The default OpenFreeMap dark and Positron styles follow Ink's colour scheme. Set `styleURL` to use another style.

```tsx
import { Screen } from "ink";
import { MapView } from "@ink/maps";

export function VehicleMap({ latitude, longitude }: {
  latitude: number;
  longitude: number;
}) {
  return (
    <Screen title="Bus location">
      <MapView
        initialCentre={{ latitude, longitude }}
        initialZoom={14}
        markers={[{ id: "bus", latitude, longitude }]}
      />
    </Screen>
  );
}
```

Place the map as the screen's sole content child; it fills the area below the screen header. Its engine is included only in apps importing the module.

While the map starts or changes style, the content area keeps Ink's background colour until the style has loaded and a complete frame has rendered. There is no connecting label. Loading failures display an error.

## React behaviour

`initialCentre` and `initialZoom` seed the map on mount. Subsequent marker changes move markers without resetting the camera or interrupting a gesture. Replacing the React key intentionally creates a fresh map.

A retained screen restores its last settled camera when uncovered. Changing `styleURL` retries style loading and clears an earlier style error.

For explicit movements, `useMap()` provides a controller accepted by `MapView.controller`. Its `moveTo({ centre, zoom })` method returns a promise. Use this for a Centre on vehicle action; routine renders do not move the camera.

```tsx
import { useMap } from "@ink/maps";

const map = useMap();
// Pass controller={map} to MapView, then call this from an action:
await map.moveTo({ centre: { latitude, longitude }, zoom: 14 });
```

When provided, `styleURL` must be an HTTPS MapLibre style URL. `initialZoom` defaults to 14; zoom levels range from 0 to 22. A controller belongs to one mounted map. The movement promise resolves when the native camera position has been applied. The Android map keeps gestures and marker rendering native; backgrounding the app pauses its rendering.

Markers have stable `id`, `latitude`, `longitude` and an optional `label`. `onMarkerPress(id)` lets the app open a detail page. `onCameraIdle({ centre, zoom })` reports the settled view when needed. Gestures and animation frames stay native.

## Scope

The map supports markers, gestures and camera movement. Clustering, offline regions, routing, geocoding, traffic and React content inside markers are not supported.

The default style needs no credentials. If you choose another source, supply its style URL and any required credentials. Attribution stays visible. Cached tiles do not guarantee offline access.

Use [Location](/location) for the phone's position. Location permission is not required merely to display supplied coordinates.
