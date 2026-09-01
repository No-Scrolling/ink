---
title: "Maps"
description: "Render a native map with camera controls and typed annotations."
---

`@ink/maps` renders an interactive native map. Use a map controller to read and move the camera, and pass markers, circles, or polylines to `MapView`.

## Show a map

Create one controller and pass it to `MapView`.

```tsx
import { MapView, mapController } from "@ink/maps";
import { Screen } from "ink";

export default function Places() {
  const map = mapController({
    initialCamera: {
      centre: { latitude: 51.5074, longitude: -0.1278 },
      zoom: 13,
    },
    style: "muted",
  });

  return (
    <Screen title="Places">
      <MapView controller={map} />
    </Screen>
  );
}
```

`style` is `"standard"` by default and also accepts `"muted"`.

The camera uses WGS84 decimal coordinates:

| Field | Values | Meaning |
| --- | --- | --- |
| `centre` | Latitude and longitude | Point at the centre of the view. |
| `zoom` | 0 to 22 | Continuous map zoom. |
| `bearing` | 0 to 360 degrees | Clockwise rotation from north. |
| `pitch` | 0 to 60 degrees | Tilt from a top-down view. |

`bearing` and `pitch` are optional and default to `0`.

## Move the camera

Using the same controller, call `move()` for a known camera or `fit()` to show a set of coordinates.

```tsx
import { Button, Stack } from "ink";

const london = { latitude: 51.5074, longitude: -0.1278 };
const greenwich = { latitude: 51.4769, longitude: 0.0005 };

<Stack gap={12}>
  <MapView controller={map} />
  <Button onPress={() => map.move({ centre: london, zoom: 15 })}>
    Show central London
  </Button>
  <Button onPress={() => map.fit([london, greenwich], { padding: 24 })}>
    Show both places
  </Button>
</Stack>
```

Camera movement is animated by default. Pass `{ animated: false }` to `move()` or `fit()` to move immediately. Ink also moves immediately when the system requests reduced motion.

The controller reports its current camera while `loading`, `ready`, or `error`. In `ready`, `moving` is `true` during gestures and animated movement. Gesture updates are throttled, followed by one final update when movement stops.

## Add annotations

Pass an annotation list to `MapView`. Annotation IDs must be unique within the view.

```tsx
import { MapView, mapController } from "@ink/maps";
import { Screen, state } from "ink";

const selected = state<string | null>(null);
const map = mapController({
  initialCamera: {
    centre: { latitude: 51.5074, longitude: -0.1278 },
    zoom: 13,
  },
});

const places = [
  {
    kind: "marker",
    id: "home",
    coordinate: { latitude: 51.5074, longitude: -0.1278 },
    label: "Home",
    icon: "home",
  },
  {
    kind: "circle",
    id: "search-area",
    centre: { latitude: 51.5074, longitude: -0.1278 },
    radiusMetres: 500,
    colour: "#3F51B5",
  },
] as const;

<Screen title="Places">
  <MapView
    controller={map}
    annotations={places}
    selectedId={selected.value}
    onAnnotationPress={(id) => selected.set(id)}
  />
</Screen>
```

| Kind | Required fields | Optional fields |
| --- | --- | --- |
| `"marker"` | `id`, `coordinate` | `label`, Material Symbol `icon` |
| `"circle"` | `id`, `centre`, `radiusMetres`, `colour` | None |
| `"polyline"` | `id`, `coordinates`, `colour`, `width` | None |

Updating an annotation with the same ID changes it in place. Removing it removes its selected appearance, but does not change the app's `selectedId` state.

You can render a route returned by another package as a polyline. `@ink/maps` does not calculate routes or geocode addresses.

## Lifecycle, permissions, and errors

`MapView` loads when its screen becomes active and pauses map work when the screen leaves or the app moves to the background. Its controller keeps the latest camera for the life of that screen instance.

One controller can be attached to one active `MapView`. Attaching it to another view returns an `already-attached` error.

Rendering a map requires no runtime permission. To show the device's position, read a fix with `@ink/location` and pass its coordinate as an annotation. The map does not request location itself.

Errors provide `kind`, `message`, and `retryable`. They distinguish unsupported rendering, invalid cameras or annotations, attachment conflicts, renderer failures, unavailable map data, offline data, resource limits, and unexpected failures.
