# Android startup and icon memory

Ink retains less icon memory and reports JavaScript readiness earlier on the Light Phone III. This run does not establish a material improvement in the phone's overall launch time. Vulkan initialisation remains on the UI thread.

Baseline: current `main`, `0885cd10498e8e8498837a7d9a4391ea87061619`.
Implementation: `285ab68b0181640323bc57d1fa777b73f0144511`.

## Results

Values are medians of 15 launches per variant per device. Changes subtract the before median from the after median. Memory uses KiB, as reported by Android. The emulator results are indicative because other agents shared the emulator.

| Metric | Device | Before | After | Change |
| --- | --- | ---: | ---: | ---: |
| `am start -W -S` TotalTime | Light Phone III | 300 ms | 297 ms | −3 ms (−1.0%) |
| JavaScript app ready | Light Phone III | 588 ms | 512 ms | −76 ms (−12.9%) |
| First content presented | Light Phone III | 608 ms | 605 ms | −3 ms (−0.5%) |
| Startup PSS | Light Phone III | 21,677 KiB | 21,124 KiB | −553 KiB (−2.6%) |
| Startup RSS | Light Phone III | 134,324 KiB | 133,724 KiB | −600 KiB (−0.4%) |
| `am start -W -S` TotalTime | Emulator | 157 ms | 147 ms | −10 ms (−6.4%) |
| JavaScript app ready | Emulator | 391 ms | 373 ms | −18 ms (−4.6%) |
| First content presented | Emulator | 423 ms | 414 ms | −9 ms (−2.1%) |
| Startup PSS | Emulator | 30,216 KiB | 29,845 KiB | −371 KiB (−1.2%) |
| Startup RSS | Emulator | 138,368 KiB | 137,988 KiB | −380 KiB (−0.3%) |
| Retained icon alpha data | Both | 590,261 bytes | 208,722 bytes | −381,539 bytes (−64.6%) |
| Core startup workload | Apple M4 Pro | 3.968 ms | 4.015 ms | +0.047 ms (+1.2%) |

The phone's median paired TotalTime difference is **+2 ms**, while its difference between variant medians is −3 ms. This disagreement reinforces the lack of a clear overall launch-time gain. The paired first-content difference is −1 ms. The paired PSS and RSS differences are −548 KiB and −600 KiB. [summary.json](summary.json) includes both calculations.

## Implementation

- Start the JavaScript thread before reading and preparing icons. Install the prepared React tree before any queued commit can be drained. No additional icon worker or Vulkan thread is introduced.
- Crop only transparent icon borders when building the React tree. Retain each mask's original dimensions and the crop offsets. Restore the full raster in a temporary buffer for its first atlas upload. This preserves geometry, filtering, alpha values and GPU atlas allocation. The v1 asset format and compiled app assets remain unchanged.
- Open the preview pack only when the generated `INK_MEDIA_LIBRARY_ENABLED` capability is true. Keep the index scan for media-library apps, but perform compaction on a worker. The worker shares immutable mappings, streams retained records to a temporary file, syncs it and atomically renames it. An insert changes the revision and cancels replacement. Readers retain valid mappings, and the directory sync runs after releasing the write lock. Interrupted temporary files are ignored and overwritten by a later compaction; the existing torn-final-record recovery remains.
- Start HTTP-engine and decoder-cache warm-ups after the first successful presentation. First HTTP demand retains the existing lazy engine path. Audio activation also starts the once-only asynchronous codec warm-up when it occurs before presentation. Decoder selection, player construction, audio processors and playback parameters are unchanged. Light SDK startup, device keys, haptics and notification handling keep their existing timing.
- Enter the existing scoped performance-core preference before QuickJS runtime/context creation, shim installation and bundle evaluation. Hold it through pending jobs, and release it when the job queue becomes idle.

System-glyph batching is skipped. This probe provides no measured benefit for an emoji/fallback-heavy screen. Moving rasterisation to workers would also need validation of stale requests, scene replacement and surface teardown. The existing glyph path is unchanged.

## Device method

The only installed application was the Counter-derived probe, `com.vandam.ink.startupprobe`. Its [source](probe.tsx) displays 28 icons; shared navigation assets bring the distinct mask count to 29. [probe.toml](probe.toml) explicitly enables network, audio playback and the media library so their startup work is included. No HTTP request or audio playback is started by the probe.

Both APKs use the same throwaway signing key, application ID, release settings, presentation instrumentation, source, bytecode, icon assets and capabilities. [artifacts.json](artifacts.json) records APK, certificate and asset hashes and device build fingerprints. The SDK's release build command was:

```sh
INK_KEYSTORE_PASSWORD=<throwaway-key-password> INK_PRESENTATION_TIMING=1 \
  scripts/ink -C .ink/startup-probe build
```

Each measured pair installs baseline and candidate APKs in that order with `adb install -r`. Each measured launch uses:

```sh
adb -s LP3LHMA531900140 shell am start -W -S \
  -n com.vandam.ink.startupprobe/com.vandam.ink.MainActivity
```

A logcat marker is written immediately before the `am` command. JavaScript readiness is the native `JavaScript app ready` log. First content is the first native `Presented` log with a non-zero scene revision. These marker-relative intervals include `am -S` force-stop overhead; they have a different origin from Android's TotalTime. `Presented` records the renderer's presentation result, not a physical display measurement. Memory is collected with `dumpsys meminfo` after a two-second settling period.

These are process-cold launches. App data and pipeline caches persist across launches; OS page caches are not flushed. The preview pack is empty. This run therefore does not quantify compaction savings for a populated pack or the benefit of skipping it in an app without the media-library capability. It also does not quantify first network-request or audio-playback latency.

The emulator was started with `emulator -avd Light_Phone_III -writable-system`. The first series stopped when `com.vandam.ink.commitpathprobe` took foreground: one candidate attempt was discarded. Its 11 completed samples remain in [emulator-initial/samples.json](emulator-initial/samples.json), excluded from the table. A new full series established the probe as foreground with an unmeasured launch before each measured `-S` launch, then checked foreground immediately before and after the launch and after settling. All 30 replacement samples passed those checks; no further discards occurred. The preparatory launch warms file caches, so emulator absolute timings should not be compared with phone timings.

[measure.ts](measure.ts) records each launch's output, PID-filtered logcat, memory report and sample values. It can resume interrupted output directories and records foreground-related discards. The phone series completed before the full emulator retry. No ADB server restart, accessibility instrumentation, display-setting change or lock-setting change was used.

## Pixel and behaviour checks

The probe was confirmed foreground before screenshots. Saved before/after screenshots are 1080 × 1240 on both devices. Pixel comparisons found **zero changed pixels in each complete frame**, including the icon region `(100, 300)–(980, 1000)`. [pixel-comparison.json](pixel-comparison.json) records dimensions, changed-pixel counts and maximum channel differences. [icon-masks.json](icon-masks.json) records the per-mask alpha bounds and byte counts.

`bun run test` passed all 177 existing tests: 18 unit, 52 compiler, 52 SDK, 29 runtime, 18 scene and 8 audio tests. No new tests were written. The release Android APK build passed. [host-result.json](checks/host-result.json), [phone-cleanup.json](checks/phone-cleanup.json) and [emulator-cleanup.json](checks/emulator-cleanup.json) record the host results and device clean-up checks. The existing host audio suite does not exercise device playback lifecycle, and this run does not directly exercise concurrent preview inserts or interrupted compaction on Android.

`bun run bench:core` passed all 30 workload correctness and idle contracts. The saved baseline was rerun against the candidate in alternating order with five processes, 20 warm-ups and 100 measured samples per workload per process. Startup's median changed from 3.968 to 4.015 ms; p95 changed from 7.015 to 5.121 ms. This host workload excludes Android process launch, platform services, GPU work and presentation, and does not establish a startup speed improvement. [core/report.md](core/report.md) contains the results. The benchmark predates the final write-lock release adjustment, which affects only preview compaction; the final full test run and measured Android APK include that adjustment.

An initial candidate benchmark attempt failed because dependency installation changed the generated fixtures' React symlink location. Those generated symlinks were corrected before the successful paired run. The first Android candidate build also caught a JNI 0.22 boolean type mismatch; the measured APK contains the correction.

## Cleanup

Both devices returned `Success` for uninstalling `com.vandam.ink.startupprobe`. Before/after `pm list packages` comparisons show that this was the only removed package, and filtered package queries confirm it is absent. [phone-cleanup.json](checks/phone-cleanup.json) and [emulator-cleanup.json](checks/emulator-cleanup.json) record the comparisons without retaining the devices' application inventories. The phone's Luma accessibility service setting is unchanged. No existing application was installed over, uninstalled or cleared.

Temporary APKs, signing keys, build directories and benchmark executable bundles were deleted after retaining the evidence in this directory. No changes were pushed.
