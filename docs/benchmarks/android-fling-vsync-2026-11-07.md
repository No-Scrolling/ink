---
title: "Android fling vsync measurements"
description: "Continuous spline fling checks on the Light Phone III and emulator."
---

Ink now evaluates fling positions from `Choreographer.FrameCallback.frameTimeNanos`, keeping sub-pixel positions until `nativeScrollBy`. Android's millisecond animation clock no longer determines each step.

## Approach

`SplineFling` implements the unbounded spline state in [Android 14 OverScroller](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/android14-release/core/java/android/widget/OverScroller.java). It retains Android's density coefficient, default friction, velocity input, integer final distance and millisecond duration rounding. The fling uses the spline, rather than the interpolator used by `startScroll`.

The implementation evaluates the underlying cubic directly instead of interpolating Android's 100-entry table. At a 6,470 px reference distance, a separate comparison at 10,001 normalised times found a maximum 0.574 px difference from that table. End distance and planned duration match exactly. Cubic reference assertions allow 0.001 px for float output and rounded nanosecond timestamps. Frame-step assertions allow 0.001 px for float subtraction.

The frame timestamp and `System.nanoTime()` share the monotonic time domain. Touch release starts the fling at `System.nanoTime()`; a row-mount resume starts at that callback's frame timestamp. Expected presentation timestamps are available through Android's newer frame timeline API, but using them would advance the sampled position by the rendering lead time. This change keeps the existing frame callback and scheduling behaviour.

The host retains edge clamping through `nativeScrollBy`, the `SCROLL_PENDING` pause and `waitingFlingVelocity` resume, velocity magnitude during waits, touch cancellation, and commit pacing. No renderer code changes.

## Release probe

Measured on 7 November 2026 against `ee69e257f7e469a33882b74dd1b1ea9bd36c6b01`. Implementation commit: `d243549cb88ffe5b3d0a74b2f70920f0a3e45cf7`. Before and after use the same app and release settings. The baseline adds timing markers only.

- Copy `examples/counter` to the ignored `.ink/flingprobe` directory.
- Set package `com.vandam.ink.flingprobe`, a throwaway signing key and its `certificate_sha256` in `ink.toml`.
- Render 300 `Text` rows at size 32 inside a `Screen` and `Stack` with gap 16.
- Build with `INK_PRESENTATION_TIMING=1`, release signing and the checkout's `scripts/ink` launcher.
- Restart the app, wait 1.6 seconds, then run `adb -s SERIAL shell input swipe 540 1000 540 300 60`.
- Capture three swipes per build and device. All measured swipes produced an initial velocity of 11,666 px/s and a planned distance of 6,161 px.

The emulator uses `emulator -avd Light_Phone_III -writable-system -vsync-rate 120 -no-snapshot-load -no-snapshot-save`. Emulator-only minimum and peak refresh settings select 120 Hz. The LP3, serial `LP3LHMA531900140`, was already at 120 Hz. Its display, refresh-rate, lock and accessibility settings were not changed. Device control used direct ADB, with no accessibility automation.

## 120 Hz results

Each run uses the first 15 differences between consecutive `Presented ... scroll_y` values after `FlingStarted`. The initial partial displacement is omitted because it has no preceding post-start position. A ratio is `max(stepₙ, stepₙ₋₁) / min(stepₙ, stepₙ₋₁)`. The median column is the median of three run medians; the maximum includes every early-step ratio across those runs. No missed-frame outliers are removed.

| Device | Build | Median ratio | Maximum ratio |
| --- | --- | ---: | ---: |
| LP3, 120 Hz | Before | 1.1160 | 1.1348 |
| LP3, 120 Hz | After | 1.0078 | 1.0300 |
| Emulator, 120 Hz | Before | 1.1213 | 2.1538 |
| Emulator, 120 Hz | After | 1.0112 | 2.0137 |

The emulator still misses some early frames and has scheduling jitter. Its maximum includes those gaps; this change does not remove them. LP3 traces remove the recurring approximately 9/8 step pattern. Driver presentation timestamps are available on the LP3; the emulator reports presentation timing unavailable, so its `Presented` timestamps are submission measurements.

Representative first 15 complete steps, in pixels, from run 2:

| Device | Build | Steps |
| --- | --- | --- |
| LP3 | Before | 93, 105, 93, 93, 92, 92, 103, 91, 90, 101, 89, 88, 98, 87, 85 |
| LP3 | After | 100.133, 97.430, 95.958, 96.230, 96.094, 95.904, 95.462, 94.849, 94.238, 93.499, 92.792, 91.849, 91.179, 90.189, 89.136 |
| Emulator | Before | 93, 92, 104, 92, 91, 103, 90, 89, 100, 88, 87, 97, 85, 84, 94 |
| Emulator | After | 97.205, 93.324, 100.577, 96.540, 96.165, 95.717, 95.197, 94.610, 93.957, 93.243, 92.471, 89.207, 93.206, 88.149, 90.578 |

## Distance and duration

| Device | Fling travel before → after | Completion before → after |
| --- | ---: | ---: |
| LP3 | 6,161 → 6,161 px | 1,485.7 → 1,510.8 ms |
| Emulator | 6,161 → 6,161 px | 1,515.9 → 1,511.1 ms |

Travel agrees within 0.002 px in the representative traces. Completion is measured from timing markers, with one additional baseline run and the median of three updated runs. The planned duration is 1,508 ms for both curves. The LP3's measured completion shifts by 25.1 ms (1.7%): this is consistent with a change in the legacy animation clock’s start phase; the new tail also continues at sub-pixel precision. The last changed integer position in the original traces precedes actual completion. These measurements support unchanged travel and decay feel, with a small timing-phase difference; they do not constitute a subjective feel assessment.

## Verification and cleanup

`bun run test` passed all 177 reported host checks. `bun run test android --serial emulator-5554` passed all 14 Android contracts, including three fling contracts. They exercise the production Kotlin class against Android's final distance, fixed reference durations, independent exact cubic points and speed magnitudes, decreasing steps at 60, 90 and 120 Hz, negative velocity, stop/restart and one-time final delivery.

An emulator release smoke check reached the bottom edge, stopped further forward flings, and scrolled backwards again. Touch-down interrupted an active fling. Virtualised-list mounting was not separately measured; its host control flow is retained.

Release smoke checks also ran on the emulator at 60 and 90 Hz. Median submission intervals were 16.668 and 11.125 ms respectively, with the same 6,161 px distance and 1,508 ms planned duration. The emulator was returned to its original 60 Hz mode, and its temporary minimum and peak refresh settings were deleted (both originally absent).

The probe was uninstalled from both devices. Complete `pm list packages` snapshots were compared with the pre-install lists; the original packages were retained. The Android contract runner also uninstalled its own `com.vandam.ink.contracttests` fixture from the emulator. LP3 accessibility service settings matched before and after. Large build outputs and throwaway signing material were deleted; raw traces are not committed.

## Limits

The cubic is the Android 14 spline, not a platform-delegated implementation. A future Android change to fling physics would need to be adopted here. Float scene coordinates retain their existing precision limits at large offsets. Emulator missed frames and scheduling jitter remain visible in the maximum ratio. A dedicated virtualised-list device probe and a subjective comparison of fling feel remain outside these measurements.
