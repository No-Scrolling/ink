# Weather tab switching — 9 September 2026

## Finding

Warm Current Location tab reveals are materially slower than Settings. The measured delay precedes native commit application; image decoding/uploading is not the bottleneck in these samples.

| Tap interval | Switches | Settings median | Current Location median | Current Location p95 |
|---|---:|---:|---:|---:|
| 250 ms | 40 | 4.13 ms | 44.54 ms | 48.51 ms |
| 100 ms | 60 | 3.87 ms | 32.44 ms | 34.71 ms |

Times are native InputUp to first subsequent Presented log (submission completion), not actual screen presentation. Emulator presentation timing is unavailable; logcat adds approximately millisecond timestamp precision. All 100 inputs dispatched and produced a frame before the next input.

Current Location dispatch-to-native-apply medians were 40.45 ms and 29.34 ms respectively. Native apply-to-commit medians were 0.90 ms and 0.77 ms. Its first renderer frame median was 0.76 ms in both runs. Across 142 frames there were **zero cache misses** (the aggregate includes glyph and image caches). Textures therefore were not repeatedly decoded/uploaded on reveal. Scene/instance buffer uploads still occur.

## Mechanism and remaining uncertainty

Tabs use React Activity hidden/visible in packages/ink/src/navigation.ts. Reveal reconnects effects. The custom renderer explicitly flushes passive effects and their synchronous corrections before posting its native commit (packages/ink/src/renderer.ts).

The weather useCurrentPlace effect restarts a current-location request on reconnection. useForecast reconnects its effect, sets loading state, starts weather and air-quality requests, and restarts its interval. Existing data is retained, so the tab does not have to await the network response to appear. Forecast reconstructs hourly row objects on render; List treats their new identities as content changes and invokes renderItem for its retained window. This creates avoidable JavaScript/list work during reveal. These are source-supported contributors, not a function-level CPU attribution: no ablation has yet separated their individual costs or image component reconciliation from other React work.

WeatherSymbol uses bundled assets. Vulkan's ImageCache retains textures across tab changes and trims only over its 8 MiB budget. Multiple displayed symbols can share one cached texture. The measured GPU resource path is inexpensive; reducing PNG count is not the first optimisation to pursue.

Recommended next investigation/fix: keep request/cache lifetimes independent of Activity effect reconnection and stabilise forecast row identities, then remeasure. Preserve renderer reveal-correction semantics until their correctness requirements are understood.

## Method and limits

Inspected the user's running dev app on emulator-5554 (1080×1240), then installed an isolated instrumented build of the working tree, experiment-20260909-091338-6f891f06, package com.vandam.ink.weatherprofile, INK_PRESENTATION_TIMING=1. Original app and data were preserved. Temporary manifest edits were restored exactly. No application implementation was changed.

The isolated app fetched fresh current-location weather; screenshots show the same layout but slightly different forecast values/icons from the existing dev session. Results characterise the current source's warm forecast reveal, not exact timings of that already-running dev binary. Build finished before measurement. Device-side Java injection alternated Settings and Current Location with 20 ms presses at 250 ms and 100 ms intervals. Timing logs and a supplementary scheduler/render trace were captured. No tests were written.

Local evidence: .agent-tools/weather-tabs/{summary.json,log-250.txt,log-100.txt,taps-250.txt,taps-100.txt,trace.txt,open.png,current.png,loaded.png,analyse.py}. First injector attempt used an old dex lacking TabTaps and failed before injecting; it is excluded. The rebuilt injector succeeded. Instrumented build and screenshot evidence are retained by agent-tools.

## Cache and row implementation

Implemented a shared in-memory resource for forecast/location requests, subscribed through useSyncExternalStore. Fresh subscriptions do not update state or restart a request. In-flight work finishes into the cache even when no tab subscribes. Timers stop while hidden; revealing stale data starts a background refresh. Failed refreshes retain existing data, stop automatic timers, and impose a one-minute backoff on subscription-triggered retries; explicit Retry bypasses freshness/backoff and shares any pending request.

Weather refresh interval is one minute from successful completion, preserving the prior interval. Current location is re-read after 15 minutes from a successful lookup, retaining the native request's existing maximumAge=15 minutes policy (this does not guarantee the underlying location fix is younger than 15 minutes throughout that interval). Different locations/units select separate forecast entries. Caches last for the app process. Hourly row derivation now has an explicit useMemo keyed by the weather data object.

React Compiler is already enabled for app source in crates/ink-compiler/src/bundle-javascript.js. It does not suppress effect reconnection or provide request caching/freshness policies. The original statement that forecast rows necessarily rebuild was too strong: compiler memoisation may already reuse them. Both changes were measured together; the results do not isolate an additional benefit from explicit row memoisation.

Candidate: experiment-20260909-092725-8c1e763c, package com.vandam.ink.weathercached. Same emulator, injector and intervals as above, with freshly fetched weather and default preferences.

| Tap interval | Switches | Settings median | Current Location median | Current Location p95 |
|---|---:|---:|---:|---:|
| 250 ms | 40 | 6.11 ms | 8.83 ms | 11.09 ms |
| 100 ms | 60 | 3.92 ms | 6.09 ms | 9.17 ms |

All 100 inputs dispatched and produced a frame before the next input. Current Location dispatch-to-apply medians fell to 3.98 ms and 2.80 ms. First run included 24 cache misses when Settings was first visited; second run had zero. The new runs produced exactly one frame per switch, versus 142 frames for 100 baseline switches. These are sequential diagnostic runs, not randomised controlled benchmarks; live forecast values and host scheduling vary. They strongly support removing reveal-triggered request/state churn, rather than optimising image decoding.

Validation: TypeScript check and instrumented Android build passed. Changed Celsius to Fahrenheit through Settings and confirmed 57° with corresponding detail values, then returned to cached Celsius (14°). The initial visual interpretation of an incomplete unit-change screenshot was incorrect; see the screenshot verification below. No tests were written. An earlier candidate failed because Ink's clearTimeout rejects undefined; guarding absent timer handles fixed startup, and the successful measurement excludes that build.

Candidate evidence: .agent-tools/weather-tabs/cached/ (logs, timings, screenshots and summary.json).

The hidden-cache check left Settings open for 68 seconds. No Ink work was logged while hidden. On return, the cached forecast submitted in about 11 ms and a background refresh completed about 1.1 seconds later. The initial report that this left incomplete content was a misinterpretation of the image preview, corrected by direct pixel inspection below.

## Screenshot verification and correction

A follow-up investigation on the same emulator initially pursued renderer and presentation faults based on that visual interpretation. Direct inspection with `scripts/agent-tools image` establishes that the saved `cached/stale-settled.png` and `cached/reveal-again.png` are **pixel-identical across the entire 1080×1240 screen**: zero changed pixels at threshold 0. The supposedly missing title is present (4,074 bright pixels in the top 120 rows), and the forecast area contains 74,701 bright pixels. Switching tabs did not restore missing content in these captures; it displayed the same content. The earlier assertion of a persistent stale-refresh rendering bug is therefore withdrawn.

Temporary renderer diagnostics (including full buffer uploads, geometry rebuilding, Vulkan validation, frame readback and a presentation fence wait) were removed. They do not constitute verified fixes. The frame-readback and screenshot evidence does not establish a presentation/composition fault either; the earlier inference to that effect was incorrect. Existing input-handling and weather caching changes remain intact. No additional production renderer change or tests were added for this investigation.

Pixel comparison evidence: `.agent-tools/image-20260909-101840-03c45c52/result.json`; forecast-region inspection: `.agent-tools/image-20260909-101840-80b20d8c/result.json`; title inspection: `.agent-tools/image-20260909-101821-2d2b3e86/result.json`. The screenshots remain under `.agent-tools/weather-tabs/cached/`. This verifies the recorded stale-refresh case; it is not a claim that every possible refresh scenario has been checked.
