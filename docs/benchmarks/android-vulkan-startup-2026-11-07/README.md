# Android Vulkan startup

Ink prepares Vulkan resources on a worker from engine creation, overlapping JavaScript startup. The UI thread checks a result channel without waiting for the worker, then attaches the Android surface. This moves instance, device, descriptor, pipeline, buffer and glyph-atlas creation out of the UI-thread surface callback.

Baseline: main `e6f055f0edf085f0dc4355f02e8b63fc34fbade0`, with identical startup instrumentation in `0b44fed2195fa0343ba1b54c10082d7f208007ac`. Implementation: `2dfbd427fc5808f891a0206e295c1d3c57494921`.

**Light Phone III measurements are still needed.** This task measures only the emulator. The earlier phone results of 512 ms for JavaScript readiness and 605 ms for first content belong to a different comparison and are not before/after results for this change. The phone was not used, even though it appeared in the ADB inventory.

## Release results

| Metric | Device | Before | After | Change |
| --- | --- | ---: | ---: | ---: |
| Counter TotalTime | Emulator | 145 ms | 128 ms | -17 ms (-11.7%) |
| Counter JavaScript app ready | Emulator | 370 ms | 376 ms | +6 ms (1.6%) |
| Counter First content presented | Emulator | 413 ms | 408 ms | -5 ms (-1.2%) |
| Template TotalTime | Emulator | 159 ms | 145 ms | -14 ms (-8.8%) |
| Template JavaScript app ready | Emulator | 396 ms | 403 ms | +7 ms (1.8%) |
| Template First content presented | Emulator | 428 ms | 420 ms | -8 ms (-1.9%) |

Android TotalTime improved more than first content. The small content gains do not establish a material improvement on the phone. Paired median first-content differences were −5 ms for Counter and −9 ms for Template. JavaScript readiness increased slightly, consistent with overlapping CPU work competing during startup.

Values are medians of 15 process-cold launches per variant for each app, alternating before/after. Changes subtract variant medians. Each pair installs the corresponding release APK with `adb install -r`. The probes copy `examples/counter` and `examples/light-template`, using `com.vandam.ink.vulkanprobe.counter` and `com.vandam.ink.vulkanprobe.template`. Both variants use the same signing key, source, bytecode, capabilities and presentation instrumentation. The Template probe additionally declares its existing transitive Wi-Fi permission and UnifiedPush exports so release APK verification succeeds.

Android `TotalTime` comes from `am start -W -S`. Other intervals begin at an immediately preceding device log marker, so they also include force-stop and command overhead. “JavaScript app ready” records the existing UI-thread drain of `Event::Ready`. The separate JavaScript wake marker records the first runtime queue notification on the JavaScript thread; it is not proof that bundle evaluation has finished. “First content presented” is the first successful `queue_present` result after the first applied commit. It is not a physical display scan-out measurement.

The emulator is `Light_Phone_III`, started with `emulator -avd Light_Phone_III -writable-system`. Its fingerprint is `Android/sdk_phone64_arm64/emu64a:14/UE1A.230829.036.A1/11228894:userdebug/test-keys`. The host is an Apple M4 Pro. The host and emulator were shared, so these results are indicative. Each measured launch has a preparatory unmeasured foreground launch; file caches and the pipeline cache remain warm. App data and OS page caches are not cleared.

The APK SHA-256 hashes were:

| Probe | Before | After |
| --- | --- | --- |
| Counter | `15c073539a725a4bf7cd2e33dcd739d166210081295d26d21befa7e14156ab10` | `a923694f6a65be1feac64c91f3700ef73acbc975c6b9fdf63fed2189d794e6f5` |
| Template | `0f11641b26439490480969aed5565d2425e934f7e7a11a86effa21fbb679b401` | `d8fa9dee44cd9cc22b311217ee06d255c035e4175f8b95abe3e165e51db76d41` |

App, worker, overlay, icon and media assets match between variants. Template's Android-generated baseline profile differs with its Kotlin bytecode.

## Startup spans

| Phase | Counter before | Counter after | Template before | Template after |
| --- | ---: | ---: | ---: | ---: |
| instance | 5.25 ms | 31.96 ms | 7.00 ms | 9.38 ms |
| device | 12.05 ms | 13.01 ms | 12.62 ms | 13.55 ms |
| pipelines | 2.05 ms | 2.26 ms | 2.06 ms | 2.45 ms |
| swapchain | 3.38 ms | 2.21 ms | 1.99 ms | 2.44 ms |
| atlas | 0.22 ms | 0.29 ms | 0.27 ms | 0.31 ms |
| renderer_prepare | — | 50.36 ms | — | 29.67 ms |

The durations use monotonic timers around each operation. Device creation includes physical-device and queue-family selection. In the baseline, that selection also checks the available surface. Pipeline creation covers the quad and text pipelines; the path pipeline remains lazy. Atlas allocation covers the initial 1 MiB glyph texture, font view and descriptor. The worker preparation span also includes cache loading, descriptors, buffers and extraction of pipeline-cache data; its duration is not the sum of the listed spans.

| Event | Counter before | Counter after | Template before | Template after |
| --- | ---: | ---: | ---: | ---: |
| Vulkan instance begins | 378 ms | 340 ms | 388 ms | 372 ms |
| First JavaScript queue wake | 367 ms | 372 ms | 383 ms | 391 ms |
| JavaScript app ready (UI drain) | 370 ms | 376 ms | 396 ms | 403 ms |
| First commit applied | 370 ms | 376 ms | 425 ms | 413 ms |
| First frame submitted | 413 ms | 408 ms | 427 ms | 420 ms |
| First content presented | 413 ms | 408 ms | 428 ms | 420 ms |

Event times are marker-relative medians. Medians from different events or phases need not describe one launch. The candidate medians place worker preparation before the first JavaScript queue notification. The baseline starts Vulkan setup from the UI-thread surface callback.

Both startup pipelines report `cache=unknown` on this emulator: its driver provides no valid pipeline-creation feedback. Cache input was present for the measured launches after the preparatory launch. [Vulkan pipeline-creation feedback](https://docs.vulkan.org/refpages/latest/refpages/source/VkPipelineCreationFeedbackFlagBits.html), when supported, reports actual application-cache hits and misses independently of file presence. No cache-hit performance claim is inferred from supplied data. An initial baseline launch with no pipeline-cache input spent 7.62 ms in instance creation, 16.06 ms in device selection/creation, 2.62 ms in swapchain creation, 2.57 ms in pipelines and 0.20 ms in atlas allocation.

## Resource ownership and lifecycle

- The worker owns the renderer exclusively until a channel transfers it to the engine. Its queues, command pools, descriptor pools and pipeline cache have no concurrent Vulkan callers. The cache writer receives copied bytes, not Vulkan handles.
- Instance/device creation, layouts, preferred-format pipelines and atlas allocation can start without a surface. Presentation support, supported colour formats, surface capabilities, extent and swapchain creation require the Android window.
- Preparation assumes `R8G8B8A8_SRGB`. Attachment validates that format and the graphics queue. Failure discards the incompatible renderer and creates a window-specific renderer on a worker, retaining its native window through cleanup.
- Pending preparation uses non-blocking channel reads and an 8 ms UI retry. Completed failures use the existing exponential recovery delay. Surface destruction cancels pending window-specific delivery. Activity destruction releases the receiver without joining the worker. An in-progress driver call must finish before its worker can release resources.
- Surface loss retains the device, pipelines, glyph atlas and image textures. Reattachment recreates only surface resources. Resize still updates the renderer's dimensions and swapchain. Initial attachment uses the existing immediate-commit scheduler when content arrived during preparation; gestures and animations retain their frame pacing.
- Android system-glyph requests and rasterisation are unchanged. Pipeline-cache data is still saved atomically in Android's code cache.

The emulator background/resume check retained the process, logged one surface attachment and one successful presentation, and created no additional Vulkan instance. Finishing the activity with Back and reopening it in the same process created one new engine and one new Vulkan instance. Rotation, forced incompatible-device/format fallback, teardown during a blocked driver call and physical-phone behaviour were not directly exercised. Their ownership paths were reviewed. Driver calls used for surface attachment and rendering still run on the UI thread; it never waits for worker completion.

## Pixels and checks

Before changing the renderer path, the saved baseline screenshot was decoded and its 1080 × 1240 dimensions and pixel distribution checked. In the final series, saved before/after screenshots for both probes have **zero changed pixels across the complete frame**, with a maximum channel difference of zero. Foreground ownership was confirmed immediately before and after capture. The measured first presentation followed the first commit; screenshot verification used the saved PNG pixels, not an image preview.

All 177 existing tests passed with `bun run test`: 18 unit, 52 compiler, 52 SDK, 29 runtime, 18 scene and 8 audio. Both exact-final release APK builds passed, and the Vulkan crate passed `cargo check -p ink-renderer-vulkan --features offscreen,presentation-timing`. No tests were added.

The renderer suite was run with `bun run bench:renderer`, using separate build/run phases. It fails at `test/performance/artwork.rs:187`: `revisits must exercise cache reconstruction after turnover`. The instrumentation-only baseline and candidate fail identically, including a retry with 120 samples and one round. The fixture creates 24 decoded 512 × 512 RGBA covers (24 MiB), while the current renderer cache budget is 64 MiB; its claim to exceed an eight-cover working set does not match that budget. This existing suite failure was not changed for the startup task.

The earlier exploratory Counter series used the normal frame scheduler after attachment. A subsequent series added immediate-commit scheduling. Neither is included in the table. The reported series uses the exact committed implementation, including the correction that returns worker failures to the error-backoff path.

## Reproduce on the phone

Keep this directory available as `BENCHMARK_TOOLS`. In a clean checkout of each revision, install dependencies and build the isolated probes. The build script uses its working directory as the SDK root and creates a throwaway signing key shared by the two output directories.

```sh
export BENCHMARK_TOOLS=/absolute/path/to/docs/benchmarks/android-vulkan-startup-2026-11-07
export ANDROID_HOME="$HOME/Library/Android/sdk"

# In the instrumentation-only baseline checkout, 0b44fed:
bun install --frozen-lockfile
bun "$BENCHMARK_TOOLS/build-probes.ts" /tmp/ink-vulkan-apks/before

# In the implementation checkout, 2dfbd42:
bun install --frozen-lockfile
bun "$BENCHMARK_TOOLS/build-probes.ts" /tmp/ink-vulkan-apks/after
```

Connect the phone and pass its ADB serial explicitly:

```sh
bun "$BENCHMARK_TOOLS/measure.ts" LP3LHMA531900140 \
  com.vandam.ink.vulkanprobe.counter \
  /tmp/ink-vulkan-apks/before/counter.apk \
  /tmp/ink-vulkan-apks/after/counter.apk /tmp/ink-vulkan-phone-counter

bun "$BENCHMARK_TOOLS/measure.ts" LP3LHMA531900140 \
  com.vandam.ink.vulkanprobe.template \
  /tmp/ink-vulkan-apks/before/template.apk \
  /tmp/ink-vulkan-apks/after/template.apk /tmp/ink-vulkan-phone-template

bun "$BENCHMARK_TOOLS/summarise.ts" \
  /tmp/ink-vulkan-phone-counter /tmp/ink-vulkan-phone-template
```

The default is 15 pairs. The script refuses to replace a pre-existing probe package, checks foreground ownership, saves PID-filtered logs and screenshots, uninstalls its probe in `finally`, and compares package inventories to confirm that only its probe was removed. It uses direct ADB commands, with no accessibility instrumentation or display/lock-setting changes. If interrupted by process termination, check the two exact package IDs and uninstall only an ID that this run installed. Keep raw evidence outside Git and delete the temporary APKs, key and generated `.ink/vulkan-startup-probes` directories when finished.

## Cleanup

All probe uninstalls returned `Success`. Package-inventory comparisons confirmed that each cleanup removed only its own exact probe ID. The final filtered `pm list packages` query found neither probe. No other application was installed over, removed or cleared. No device accessibility service, display setting or lock setting was changed. Large temporary outputs, including the private cloned Cargo target directory, were removed after verification.

Only this worktree was changed. Runtime bytecode compilation and CLI code were not modified. No changes were pushed. Raw logs, screenshots, APKs, signing keys and temporary build outputs are not committed; this directory retains only the report and reproduction scripts.
