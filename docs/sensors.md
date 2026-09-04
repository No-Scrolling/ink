---
title: "Sensors"
description: "Bounded native sensor observations."
tag: "Design specification"
---

`@ink/sensors` exposes available motion and environmental sensors. Check availability before presenting a feature; the API does not imply that every device contains every sensor.

```tsx
import { useState } from "react";
import { Text, useVisibleEffect } from "ink";
import { sensors } from "@ink/sensors";

export function Heading() {
  const [heading, setHeading] = useState<number | null>(null);
  useVisibleEffect(() => sensors.heading.subscribe(
    { interval: 250 },
    sample => setHeading(sample.degrees),
  ), []);
  return <Text>{heading === null ? "Waiting for heading" : `${Math.round(heading)}°`}</Text>;
}
```

The example assumes availability and permission have been handled by its parent flow. Subscriptions return cleanup functions and release while hidden. Samples include timestamps and quality information where the sensor provides it. Requested intervals are hints, not timing guarantees.

Native processing should perform filtering, fusion and high-rate aggregation. Publish only the rate needed for UI. Latest-value snapshots may coalesce; an app requiring every sample must choose a bounded recording interface with explicit overflow behaviour.

Persist recordings through a native file sink when supported. Do not accumulate an unbounded JavaScript array or use a UI subscription as a background recorder. Heading can be unreliable near interference; use the reported quality when displaying it.
