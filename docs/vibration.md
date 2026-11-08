---
title: "Vibration"
description: "Play touch feedback and alarm vibration patterns."
---

Use `@ink/vibration` to vibrate for a duration or play a pattern. Durations use non-negative integer milliseconds.

## Vibrate for a duration

```ts
import { vibrate } from "@ink/vibration";

await vibrate(200);
await vibrate(200, { strength: 0.4 });
```

Omit `strength` to use Android's default amplitude. Set it from `0` (off) to `1` (maximum). Ink maps this range to Android's amplitudes from 0 to 255. On devices without amplitude control, Ink approximates strength by switching the motor on for that fraction of each segment, then off for the rest.

On LightOS, touch vibrations do nothing when the user turns off haptics. Outside LightOS, Android controls vibration behaviour.

## Play a pattern

```ts
import { vibrate } from "@ink/vibration";

await vibrate([
  { ms: 300, strength: 1 },
  { ms: 150, strength: 0 },
  { ms: 300, strength: 0.5 },
]);
```

Each segment supplies its duration and strength. A new vibration replaces the current one. Patterns play once. The returned promise resolves when Android receives the request, without waiting for the vibration to finish. Devices without a vibrator do nothing.

## Vibrate for an alarm

```ts
import { vibrate } from "@ink/vibration";

const pattern = [
  { ms: 500, strength: 1 },
  { ms: 250, strength: 0 },
  { ms: 500, strength: 1 },
];
await vibrate(pattern, { alarm: true });
```

The default usage is Android's `USAGE_TOUCH`. Set `alarm: true` for an alarm or timer the user set. This uses `USAGE_ALARM`, bypasses the LightOS haptics preference and allows vibration while the screen is off. Android's alarm settings still apply.

Android only allows vibration from the foreground, except for alarms and foreground services. Calling `vibrate` does not schedule an alarm or start a service. Ink background workers do not expose this module.

## Stop vibrating

```ts
import { stopVibrating } from "@ink/vibration";

await stopVibrating();
```

## Permissions

| Permission | Required for |
| --- | --- |
| `VIBRATE` | Playing vibrations |

Ink adds this permission when your app uses `@ink/vibration`. Android grants it automatically; you do not need to request access.
