---
title: "Sensors"
description: "Read sampled motion, orientation, environmental, and step sensor values."
---

`@ink/sensors` provides rate-limited streams for common device sensors. Streams use portable units and publish only their latest sample.

## Read a sensor

Declare a stream on the screen that needs it:

```tsx
import { sensorStream } from "@ink/sensors";
import { Text } from "ink";

const motion = sensorStream("accelerometer", { frequencyHz: 10 });

<Text>X {motion.sample?.x ?? 0} m/s²</Text>
<Text>Y {motion.sample?.y ?? 0} m/s²</Text>
<Text>Z {motion.sample?.z ?? 0} m/s²</Text>
```

`frequencyHz` accepts 1 to 100 and defaults to 10. `actualFrequencyHz` reports the measured publication rate once the stream becomes active.

Each stream exposes:

| Field | Meaning |
| --- | --- |
| `status` | `starting`, `active`, `paused`, or `error`. |
| `sample` | The latest sample, or `null` before one arrives. |
| `sequence` | Increments for every published sample, including equal values. |
| `actualFrequencyHz` | The measured rate, or `0` before sampling starts. |

The stream keeps no history. Store only the values your app needs rather than copying every sample into state.

## Choose a sensor

`sensorStream()` accepts these kinds:

| Kind | Sample | Unit |
| --- | --- | --- |
| `"accelerometer"` | `x`, `y`, `z`, `accuracy` | Metres per second squared, including gravity. |
| `"linear-acceleration"` | `x`, `y`, `z`, `accuracy` | Metres per second squared, excluding gravity. |
| `"gyroscope"` | `x`, `y`, `z`, `accuracy` | Radians per second. |
| `"magnetic-field"` | `x`, `y`, `z`, `accuracy` | Microteslas. |
| `"rotation"` | `x`, `y`, `z`, `w`, `accuracy` | Unit quaternion. |
| `"light"` | `value`, `accuracy` | Lux. |
| `"pressure"` | `value`, `accuracy` | Hectopascals. |
| `"proximity"` | `value`, `accuracy` | Centimetres. |
| `"step-counter"` | `stepsSinceBoot` | Steps since the last device boot. |

Every sample includes `timestampMs` as Unix time in milliseconds. Accuracy is `"unreliable"`, `"low"`, `"medium"`, or `"high"` where the sensor reports it.

Vector axes use a portrait device frame. Positive X points right, positive Y points towards the top edge, and positive Z points out of the screen.

## Request activity permission

Most sensors need no runtime permission. Step counting needs activity-recognition permission on devices that require it.

```tsx
import { activityPermission } from "@ink/sensors";
import { Button, Text, match } from "ink";

const activity = activityPermission();

{match(activity, {
  loading: () => <Text>Checking activity access</Text>,
  ready: (result) => <Text>{result.value}</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
<Button onPress={() => activity.request()}>Allow activity access</Button>
```

Creating the permission resource or a step stream never opens a prompt. Call `request()` from a user action.

## Lifecycle and errors

A stream starts when its screen becomes active. Moving the app to the background pauses it. Returning to the same screen resumes sampling and keeps the previous sample until a new one arrives. Leaving the screen stops the stream completely.

When several streams use the same physical sensor, each still publishes at its requested rate.

Errors distinguish denied or blocked permission, unavailable hardware, unsupported rates, interruption, invalid samples, and unexpected failures. Every error has `kind`, `message`, `retryable`, and the requested sensor kind.

## Accessibility

Do not announce every sensor sample. Present a throttled text summary when assistive technology needs the value.

Do not make motion, orientation, light, or proximity the only way to complete a task. Provide an equivalent button or other direct control.
