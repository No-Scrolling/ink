---
title: "Maps"
description: "Render an attributed native map with camera controls and typed annotations."
tag: "Planned"
---

`@ink/maps` renders an interactive native map. The module owns renderer lifecycle, map-data policy, attribution, and preview behaviour while callers work with a camera and annotations.

## Show a map

Create one session and pass it to `MapView`:

```tsx
import { MapView, mapSession, standardMap } from "@ink/maps";
import { Screen } from "ink";

const map = mapSession({
  content: standardMap({ style: "muted" }),
  initialCamera: {
    centre: { latitude: 51.5074, longitude: -0.1278 },
    zoom: 13,
  },
});

<Screen title="Places">
  <MapView session={map} />
</Screen>
```

`standardMap()` uses Ink's documented production map-data provider and deterministic preview adapter. `MapView` renders the required attribution and licence link; callers cannot hide it.

Third-party source modules can export another opaque `MapContent` with its own credentials, style rules, attribution, online policy, and preview adapter. Callers never receive tile URLs or provider credentials.

## Move the camera

Call `move()` for a known camera or `fit()` to show coordinates:

```tsx
<Button onPress={() => map.move({ centre: london, zoom: 15 })}>
  Show central London
</Button>
<Button onPress={() => map.fit([london, greenwich], { padding: 24 })}>
  Show both places
</Button>
```

The camera uses WGS84 decimal coordinates. `zoom` accepts 0 to 22, `bearing` 0 to 360 degrees, and `pitch` 0 to 60 degrees. Movement is animated unless disabled or the system requests reduced motion.

The session publishes its latest camera, movement state, content availability, and errors. Gesture updates are coalesced, followed by one final snapshot when movement stops.

## Add annotations

Pass markers, circles, or polylines to the view:

```tsx
<MapView
  session={map}
  annotations={places}
  selectedId={selected.value}
  onAnnotationPress={(id) => selected.set(id)}
/>
```

Annotation IDs must be unique. Updating an annotation with the same ID changes it in place. Map does not acquire location, geocode addresses, calculate routes, or persist favourites; compose those domain operations from their owning modules.

## Use offline content

A `MapContent` declares whether it is online-only or supports a durable offline region. Downloading a region uses Downloads and returns an opaque content reference. Provider limits, expiry, maximum area, and attribution remain part of that content module's interface.

## Lifecycle, accessibility, and errors

`MapView` fills its available content area unless given explicit width and height. It pauses rendering when hidden or backgrounded. One active view can attach to a session.

Rendering requires no location permission. Map's native semantic tree exposes visible labelled markers and declared map actions. It honours reduced motion and text scaling.

Errors distinguish unsupported rendering, invalid cameras or annotations, attachment conflicts, unavailable or offline map data, provider authentication, resource limits, and unexpected failures. Every error provides `kind`, `message`, and `retryable`.
