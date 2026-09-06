# Separately loaded web runtime on LP3

6 September 2026. **The split reduced median idle PSS by 5.16 MiB for the counter and 4.91 MiB for the unchanged, non-virtualised 1,000-row list.** Both builds already include lazy web globals and startup GC/Android allocator purging. QuickJS remains the runtime.

## Measurements

Physical Light Phone III, Android 14, ARM64 release APKs with the benchmark signing key. Three fresh processes per build and fixture; control/split order alternates each round. Samples are taken two seconds after `am start -W` returns. Every accepted sample records a foreground-window check for the expected package. The earlier exploratory runs encountered foreground interference and are retained separately, excluded from these medians.

| Fixture | Current lazy globals + GC/purge | Separately loaded web code | Reduction |
| --- | ---: | ---: | ---: |
| Counter | 31.04 MiB | 25.88 MiB | 5.16 MiB (16.6%) |
| Non-virtualised 1,000-row list | 40.50 MiB | 35.58 MiB | 4.91 MiB (12.1%) |

[Raw accepted samples](split-web-lp3-2026-09-06/verified/), [summary](split-web-lp3-2026-09-06/summary.json), [APK hashes and bundle sizes](split-web-lp3-2026-09-06/artifacts.txt), [measurement script](split-web-lp3-2026-09-06/measure.ts).

The list fixture was not edited: it still mounts every row through `Stack` and `items.map`. It rendered and scrolled on LP3 after splitting. [Screenshot](split-web-lp3-2026-09-06/scroll-lp3.png). Published README comparison figures remain unchanged; these are a small targeted experiment, not a complete benchmark rerun.

## Implementation

Release applications now use separate web packaging by default; `INK_SPLIT_WEB=0` selects the earlier single-bundle packaging. The compiler emits `ink-assets/ink-web.js` separately and replaces the bundled web dependency with a synchronous first-use loader. Counter `app.js` falls from 491,400 to 137,509 bytes; the optional file is 353,824 bytes. Total JavaScript stays approximately the same size. The saving comes from not reading, parsing and compiling unused web code at startup, rather than deleting web APIs.

On first access, the JS thread asks Android to read the packaged asset, evaluates its factory in the existing QuickJS context and supplies the app's native module. This preserves the existing pending-request map and event listeners instead of creating a second native bridge. The web exports are cached and the temporary factory global is removed. Subsequent property reads use the normal writable global values.

The initial measurements used an opt-in flag; following verification, the same split became the default for release applications. Development and worker bundles retain their existing single-bundle behaviour. The Android loader reads the APK's fixed web asset; this is not a general dynamic-module API or a loader for downloaded development bundles.

## First-use checks and trade-off

A separate counter smoke app accessed an internationalised URL, Headers, Blob text and WebSocket, then fetched `https://example.com`. On both LP3 and the ARM64 emulator, the URL resolved to `xn--mnich-kva.example`, Blob returned `hello`, and fetch returned HTTP 200 with the expected content. The emulator counter also incremented after web initialisation, checking that UI events still reached the app. [LP3 log](split-web-lp3-2026-09-06/web-smoke.log), [emulator log](split-web-lp3-2026-09-06/web-smoke-emulator.log), [counter screenshot](split-web-lp3-2026-09-06/web-counter-emulator.png), [smoke source](split-web-lp3-2026-09-06/web-smoke.tsx).

LP3 log timestamps put the first URL access, including loading and initialisation, at approximately 130 ms in one run. This is an indicative first-use cost, not a timing benchmark. Loading any web export still initialises the entire optional group, and applications that use it will eventually pay its memory cost. An after-networking sample is retained for context but is not comparable to idle counter measurements: it includes web initialisation, networking and different timing.

Release builds of both benchmark fixtures and the smoke app passed, as did TypeScript checking, `cargo check -p ink-runtime -p ink-compiler`, runtime formatting and diff whitespace checks. No tests were added. General web API conformance, first-use latency distributions, background/resume behaviour and peak memory have not been established by this spike.

## Reproduction

From the repository root, with dependencies, Android SDK/NDK and benchmark signing configured:

```sh
INK_KEYSTORE_PASSWORD=android INK_KEY_PASSWORD=android ./scripts/ink -C benchmarks/apps/ink-counter build
INK_KEYSTORE_PASSWORD=android INK_KEY_PASSWORD=android ./scripts/ink -C benchmarks/apps/ink-scroll build
```

Copy the resulting APKs to a separate directory as `split-counter.apk` and `split-scroll.apk`. Rebuild with `INK_SPLIT_WEB=0` and copy those as `control-counter.apk` and `control-scroll.apk`. Then run:

```sh
ANDROID_SERIAL=LP3LHMA531900140 bun benchmarks/results/split-web-lp3-2026-09-06/measure.ts /path/to/apks /path/to/new-results
```

The script installs the benchmark APKs, wakes the phone, dismisses its unsecured system keyguard, launches and measures fresh processes, and stops each measured app. It aborts if another app takes the foreground. It does not uninstall the benchmark apps. The temporary web smoke app was removed after verification.
