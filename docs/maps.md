---
title: "Maps"
description: "An optional MapLibre map with markers and native gestures."
tag: "Planned"
---

> **Not implemented yet.** This page defines the intended interface.

`@ink/maps` integrates MapLibre with Ink's renderer. It owns gestures, tile loading and marker rendering. Apps supply a style URL, coordinates and actions. Choosing a tile source does not require a separate Ink provider package.

```tsx
import { Screen } from "ink";
import { MapView } from "@ink/maps";

export function VehicleMap({ styleURL, latitude, longitude }: {
  styleURL: string;
  latitude: number;
  longitude: number;
}) {
  return (
    <Screen title="Bus location">
      <MapView
        styleURL={styleURL}
        initialCentre={{ latitude, longitude }}
        initialZoom={14}
        markers={[{ id: "bus", latitude, longitude }]}
      />
    </Screen>
  );
}
```

The map fills the available content area. Its engine is included only in apps importing the module.

## React behaviour

`initialCentre` and `initialZoom` seed the map on mount. Subsequent marker changes move markers without resetting the camera or interrupting a gesture. Replacing the React key intentionally creates a fresh map.

For explicit movements, `useMap()` provides a controller accepted by `MapView.controller`. Its `moveTo({ centre, zoom })` method returns a promise. Use this for a Centre on vehicle action; routine renders do not move the camera.

Markers have stable `id`, `latitude`, `longitude` and an optional `label`. `onMarkerPress(id)` lets the app open a detail page. `onCameraIdle({ centre, zoom })` reports the settled view when needed. Gestures and animation frames stay native.

## Scope

The initial scope is Buses' needs: one map, moving markers and explicit centring. Clustering, offline-region downloads, routing, geocoding, traffic and arbitrary React content inside markers are outside this scope.

Apps configure the style and required credentials. Attribution stays visible; tile access and caching follow the source's terms. A tile cache is not an offline guarantee.

Use [Location](location.md) for the phone's position. Location permission is not required merely to display supplied coordinates.
