# Android size reduction

Release APKs are smaller without changing app behaviour. The platform splash
replacement was rejected after a launch screenshot exposed a transient system-bar
difference. The original splash implementation remains in the final builds.

Measured on 7 November 2026 from baseline `0885cd10498e8e8498837a7d9a4391ea87061619`
and this worktree before the implementation commits. The SDK uses Bun 1.4.4,
Rust 1.96.0, QuickJS-ng 0.17.0, AGP 8.13.2 and arm64-v8a/API 34 release builds.
The host is an Apple M4 Pro. See the JSON files for APK hashes, entry hashes,
compressed lengths, individual experiments and benchmark samples.

The table entries are uncompressed ZIP lengths. APK is the complete signed file
length. Bytecode compression and native alignment mean the entry changes do not
add up to the APK change. `libink_android.so` is stripped. The Template also ships
11,628,960 bytes of other native libraries; their hashes and sizes are unchanged.
A zero denotes an asset that the app does not ship.

## Counter

| Item | Before bytes | After bytes | Change |
| --- | ---: | ---: | ---: |
| APK | 2,913,590 | 2,709,446 | -204,144 |
| libink_android.so | 2,745,008 | 2,541,672 | -203,336 |
| DEX | 104,252 | 104,252 | +0 |
| resources.arsc | 3,540 | 3,332 | -208 |
| app.jsbc | 58,050 | 57,135 | -915 |
| Worker bytecode | 0 | 0 | +0 |
| ink-web.jsbc | 0 | 0 | +0 |

## Light Template

| Item | Before bytes | After bytes | Change |
| --- | ---: | ---: | ---: |
| APK | 34,009,149 | 33,866,505 | -142,644 |
| libink_android.so | 3,367,736 | 3,362,560 | -5,176 |
| DEX | 3,382,100 | 3,382,100 | +0 |
| resources.arsc | 242,268 | 196,968 | -45,300 |
| app.jsbc | 353,770 | 353,772 | +2 |
| Worker bytecode | 233,714 | 32,335 | -201,379 |
| ink-web.jsbc | 170,450 | 163,159 | -7,291 |

## Inkify copy

| Item | Before bytes | After bytes | Change |
| --- | ---: | ---: | ---: |
| APK | 8,749,253 | 8,649,853 | -99,400 |
| libink_android.so | 6,428,368 | 6,356,408 | -71,960 |
| DEX | 1,896,808 | 1,896,808 | +0 |
| resources.arsc | 153,380 | 129,628 | -23,752 |
| app.jsbc | 209,844 | 208,925 | -919 |
| Worker bytecode | 0 | 0 | +0 |
| ink-web.jsbc | 170,450 | 163,159 | -7,291 |

## Retained experiments

1. **Share the worker web runtime.** Template worker bytecode falls from 233,714
   to 65,712 bytes: **168,002 bytes saved**. A worker shares the lazy asset when
   the app metadata declares it. Standalone worker bundles retain inline web code.
   Android workers keep their own asset-manager reference and load the asset only
   when a web global is first used.
2. **Use pure worker imports.** The next Template worker build falls from 65,712
   to 32,335 bytes: **33,377 bytes saved**. `ink/resource`,
   `@ink/notifications/commands` and `@ink/lightos/push/delivery` avoid UI hooks.
   Existing public imports remain available. Push capability declarations cover
   both the hook and delivery modules. Template data and worker imports use the
   new entry points. Changing the named imports alone does not remove hooks from
   the original barrels; workers must select these pure entry points.
3. **Optimise JNI glue for size.** Only `jni` uses package-level `opt-level = "s"`.
   It saves **12,408 native bytes in Counter** and **40,208 in Inkify** against
   otherwise identical profile settings. The generated app manifest copies the
   override. The global profile, QuickJS C build, engine, renderer, runtime,
   protocol, JSON, text and hashing dependencies remain at `opt-level = 3`.
   No unmeasured overrides were retained.
4. **Gate optional native UI code.** After the JNI override, the final library
   saves another **190,928 bytes in Counter** and **31,752 in Inkify**. Optional
   node payloads, list/input/canvas/player collections, composite implementations,
   layout arms and Android image-renderer caches compile only when needed.
   Video retains the shared playback layout and progress path. Canvas retains its
   own progress path. Public scene/JNI result shapes remain compatible. Static
   host inference covers the newly gated kinds; dynamic component calls keep the
   previous UI fallback without acquiring camera, maps or video permissions.
5. **Compact resource entries.** An isolated AAPT2 pass saves **208 bytes in
   Counter**, **45,300 in Template** and **23,752 in Inkify**. The final signed
   APK resource tables exactly match those isolated lengths. A linker flag alone
   was discarded by resource shrinking, so the retained Gradle action compacts
   the final `optimizeReleaseResources` output with AGP's own AAPT2 binary before
   packaging, alignment and signing. It does not enable sparse encoding.
6. **Drop unused link TLD initialisation.** Annotating `unpackTlds()` as pure saves
   **915 Counter app bytecode bytes** and **919 Inkify app bytecode bytes**. The
   Template uses conversation links and retains the initialiser. Module splitting
   changes its app bytecode by **+2 bytes**.
7. **Replace URL-only Web IDL conversions.** The focused shim reduces shared web
   bytecode from 170,450 to 163,159 bytes: **7,291 bytes saved** in Template and
   Inkify. It preserves string-hint coercion, symbol errors, realm constructors,
   caller context and lone-surrogate replacement. Sixty-eight ad hoc coercion and
   error comparisons matched `webidl-conversions` 7.0.0. Both existing split and
   inline QuickJS URL fixtures pass, including the 1,014 frozen parsing/setter
   cases. No URL throughput claim is made from the size measurement.
8. **Cache bundler metadata and skip icon AST work.** Ten alternating Counter
   builds have identical output hashes. Median time falls from **246.5 ms to
   228.8 ms**, a **17.7 ms** reduction on this host. This changes build time,
   not APK content.

These are incremental component measurements, not additive APK savings.
`experiments.json`, `jni.json`, `gated.json`, `compact-only.json`,
`icons-bundler.json`, and the complete before/after entry lists preserve the data.
`canvas.json` records the earlier, smaller Canvas-only gating step. `compact.json`
is an intermediate unsuccessful linker-flag attempt; its Inkify build overlapped
the splash experiment and is not used to attribute resource or splash savings.

## Rejected platform splash replacement

The candidate removed the AndroidX dependency and used platform
`SplashScreen.setOnExitAnimationListener`, immediately removing the view. The
Counter candidate reduced DEX by 3,600 bytes. However, repeatable saved screenshots
showed a transient system-bar difference: **4,720 changed pixels** in the top
60-pixel band. Replicating the theme inheritance and system-bar restoration did
not establish identical launch behaviour. The change was reverted.

The retained final Counter screen matches the original at **all 1,339,200 pixels**
(1080 × 1240). `splash-pixels.json` records the comparisons, and the PNG files
preserve the original, rejected candidate and final screen. Each capture checked
that the measurement application was in the foreground. `splash.json` records the
rejected candidate before final compaction; do not treat it as a final build.

## Verification

`bun run test` passes all 177 checks: 18 unit, 52 compiler, 52 SDK, 29 runtime,
18 scene and 8 audio. No new tests were added. The runtime suite exercises the
production split/inline web fixture and its frozen URL results. The compiler suite
checks capability inference and split asset removal. `final-bundle-verification.json`
confirms that the final inference adjustment leaves the measured app/worker/overlay
programs and app capability sets unchanged.

`cargo check -p ink-core` passes with defaults, without defaults, and with each
of `ui-lists`, `ui-playing`, `ui-messages`, `ui-canvas`, `text-input`, `camera`,
`video` and `maps` enabled alone. The no-feature build is warning-free. Canvas-only
builds retain one unused playback-clock field warning. Android release APK builds
compile both the minimal and feature-heavy native paths.

Two `bun run bench:core --baseline ...` comparisons use the saved original
executable and fixtures in alternating order: five rounds with 100 samples and
20 warm-ups, then seven rounds with 200 samples and 40 warm-ups. All 30 workloads
and idle contracts pass. There is no repeatable median regression across the two
runs. In the second run, controlled input changes by -0.98%, loaded input by
-4.20%, and startup by +0.25%. The apparent decoded-image increase is 0.292 µs
(+13.7%); the first comparison moved in the opposite direction (-15.6%). Row
reordering similarly changes from -5.0% to +2.6%. Tail timings are noisy under
shared-host load. The reports retain every result rather than hiding outliers.

Headless benchmarks exclude Android JNI execution, process launch, GPU work and
presentation. They do not establish device performance equivalence for those
paths. The engine and hot dependencies retain their speed profile.

Counter and Inkify pass the CLI release verifier. Template builds and signs, but
both baseline and final CLI verification reject the existing transitive
`android.permission.ACCESS_WIFI_STATE` permission. The final Template separately
passes APK signature verification and 16 KiB native-library alignment checks.
No permission declaration was changed to bypass that failure.

## Reproduction and cleanup

All builds use `ANDROID_HOME=$HOME/Library/Android/sdk` and the worktree launcher:

```sh
INK_KEYSTORE_PASSWORD=<temporary-password> scripts/ink -C examples/counter build
INK_KEYSTORE_PASSWORD=<temporary-password> scripts/ink -C examples/light-template build
INK_KEYSTORE_PASSWORD=<temporary-password> scripts/ink -C /tmp/ink-size-inkify-f46b1e45 build
bun run test
bun run bench:core --baseline <saved-baseline-run>
```

The APK measurements temporarily used `com.vandam.inksize.counter`,
`com.vandam.inksize.template` and `com.vandam.inksize.inkify`, with one throwaway
keystore, alias and certificate fingerprint in their measurement configurations.
The example configurations were restored before committing. Inkify was copied
from `/Users/vandam/Developer/inkify` to `/tmp`; its `ink` and `@ink/*` dependencies
and overrides used worktree `file:` paths, followed by `bun install` and explicit
package symlinks. The original Inkify directory was never modified.

Only `com.vandam.inksize.counter` was installed, on `emulator-5554`. It was removed.
Immediate `pm list packages` inventories changed from 196 to 195 entries, with
exactly that package removed and none added. No app was installed on the LP3, and
no display or lock settings were changed. Device checks stopped using the LP3
when it was unplugged. See `device-cleanup.json`.

Temporary APKs, signing material, the Inkify copy and large build outputs were
removed after recording the evidence. The resource action uses the internal
`OptimizeResourcesTask` API of the pinned AGP 8.13.2; recheck it when upgrading AGP.

## Implementation commits

- `d70e58f`: Worker sharing, pure imports, URL conversions and bundler work.
- `987f30c`: JNI package optimisation.
- `85b6025`: Optional native UI and renderer state.
- `db6c879`: Final resource compaction.
