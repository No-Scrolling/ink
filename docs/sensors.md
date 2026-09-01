---
title: "Sensors"
description: "Read sampled motion, orientation, environmental, and step values."
tag: "Planned"
---

`@ink/sensors` provides screen-owned sensor sessions with explicit latest-value delivery. Values use portable units and monotonic timestamps.

## Read a sensor

Declare a session on the screen that needs it:

```tsx
const motion = sensor("accelerometer", { frequencyHz: 10 });

<Text>X {motion.sample?.x ?? 0} m/s²</Text>
<Text>Y {motion.sample?.y ?? 0} m/s²</Text>
<Text>Z {motion.sample?.z ?? 0} m/s²</Text>
```

`frequencyHz` accepts 1 to 100 and defaults to 10. The session phases are `starting`, `active`, `paused`, and `error`. It publishes only its latest sample because intermediate raw sensor samples are replaceable.

Each sample includes `elapsedRealtimeMs`, a monotonic timestamp suitable for measuring intervals. Optional `capturedAtMs` is wall-clock Unix time and must not be used for duration calculations. `sequence` increments for every published sample, including equal values.

## Choose a sensor

Supported kinds include accelerometer, linear acceleration, gyroscope, magnetic field, rotation, light, pressure, proximity, and step counter. Vector axes use the portrait device frame and report documented SI units.

Accuracy is `"unreliable"`, `"low"`, `"medium"`, or `"high"` where Android supplies it. `actualFrequencyHz` reports the measured publication rate after sampling starts.

## Reduce samples natively

Use a reduction when an app needs a threshold or window summary rather than raw samples:

```tsx
const movement = sensor("accelerometer", {
  frequencyHz: 50,
  publish: {
    everyMs: 250,
    reduction: "root-mean-square",
  },
});
```

Supported reductions are defined per sensor and run before values cross into the Ink value graph. This avoids copying high-rate history into app state.

## Request activity permission

Most sensors need no runtime permission. The step counter exposes its own permission state and `request()` command. Creating a sensor never opens a prompt.

## Lifecycle, errors, and accessibility

The session starts when its screen becomes active, pauses in the background, and stops when the screen leaves. Returning to the same screen keeps the previous sample until a new one arrives.

Errors distinguish denied or blocked permission, unavailable hardware, unsupported rates or reductions, interruption, invalid samples, and unexpected failures. Every error has `kind`, `message`, `retryable`, and the sensor kind.

Do not announce every sample or make a sensor the only way to complete a task. Present a throttled summary and an equivalent direct control where appropriate.
