# Android/Rust prototype architecture

Research date: 29 August 2026. This note focuses on the first vertical slice: an Android-only Ink host that renders and updates a counter, while leaving clean seams for the compiler, Light SDK and Light keyboard.

## Recommendation

Use a conventional, very small Kotlin Android host around a Rust `cdylib`:

```text
MainActivity / FrameLayout
        |
        +-- InkView (Android Canvas, input, accessibility)
        |        |
        |       JNI
        |        |
        |   Rust Ink engine
        |   state -> layout -> hit testing -> display list
        |
        +-- future Lp3RawKeyboardView overlay
```

The Rust engine should be the deep module. Its interface should deal in app bundles, viewport changes, input events and display lists, not individual Android drawing calls. Kotlin should remain the Android adapter: it owns the `Activity`, `View`, `Canvas`, `Paint`, `Typeface`, lifecycle and eventual Compose keyboard view.

This is preferable to `NativeActivity` for Ink. Android notes that a native activity still runs inside the app VM and uses JNI for framework access; using a normal activity makes the Android-specific requirements—Canvas, accessibility, splash handling, Binder and a Compose-backed keyboard—direct rather than reconstructing them through JNI ([NDK concepts](https://developer.android.com/ndk/guides/concepts)).

## Counter vertical slice

The smallest meaningful implementation is:

1. `MainActivity` calls `installSplashScreen()` before `super.onCreate()`, then installs a full-screen `FrameLayout` containing `InkView`.
2. `InkView` creates one native engine handle, forwards size changes and pointer/key events, and releases the handle when the activity is destroyed.
3. Rust owns the counter value, button hit region, tree/layout result and display list.
4. `InkView.onDraw(Canvas)` asks for the current display list and executes it with cached `Paint` and `Typeface` objects.
5. A completed tap is sent to Rust. If the engine reports a visual change, Kotlin calls `invalidate()`.

Android's custom-view contract is a good fit: `onDraw()` receives a `Canvas` capable of drawing text, bitmaps and primitives, while `onTouchEvent()` receives pointer input ([custom drawing](https://developer.android.com/develop/ui/views/layout/custom-views/custom-drawing), [interactive custom views](https://developer.android.com/develop/ui/views/layout/custom-views/making-interactive)). `invalidate()` schedules a future draw and must be called on the UI thread; `postInvalidate()` is the cross-thread alternative ([`View.invalidate`](https://developer.android.com/reference/android/view/View#invalidate())).

Keep every UI-facing engine call on Android's UI thread for this prototype. Do not retain a `JNIEnv`: Android documents that it is thread-local. If background resources arrive later, post the result to the view and enter Rust from the UI thread ([JNI tips](https://developer.android.com/ndk/guides/jni-tips)).

### JNI seam

Keep the external seam to approximately these operations:

- create/destroy an opaque engine handle;
- update the viewport;
- dispatch a compact input event;
- write the current display list into a reusable direct buffer.

Do not make one JNI call per text or rectangle. Android explicitly recommends minimising both the amount and frequency of marshalled data and keeping JNI code in a few identifiable locations ([JNI tips](https://developer.android.com/ndk/guides/jni-tips)). A Kotlin-allocated reusable direct buffer is adequate for the prototype and avoids allocating a new `ByteArray` every frame. The event-driven, non-animated UI means the display list only needs rebuilding after state, resource or viewport changes.

Use `JNI_OnLoad` plus `RegisterNatives` when the first seam settles. Android recommends explicit registration because errors are caught at load time and only `JNI_OnLoad` needs to be exported; hidden visibility also reduces symbols and collision risk ([JNI tips](https://developer.android.com/ndk/guides/jni-tips)).

Text measurement is the first likely pressure on this seam. The Android adapter should ultimately batch measurement requests and return metrics produced by the same `Paint`/`Typeface` used to draw. Avoid a JNI round trip for every text node. The counter can start with fixed layout so this does not delay the vertical slice.

## Lifecycle and drawing rules

The host needs only `onCreate`, `onResume`, `onPause` and `onDestroy` forwarding at first. Android defines six core lifecycle callbacks and may destroy the process after the activity is stopped, so the engine must not assume an in-memory handle survives process death ([activity lifecycle](https://developer.android.com/topic/libraries/architecture/views/activity-lifecycle-views)). The counter may reset for the prototype; persistent application state can later be loaded when a new engine is created.

Avoid allocations in `onDraw`, cache `Paint`/`Typeface`, and invalidate only when dirty. Android's guidance identifies allocations in `onDraw`, unnecessary invalidations and repeated layout traversal as the main custom-view costs ([optimising custom views](https://developer.android.com/develop/ui/views/layout/custom-views/optimizing-view)).

Although it need not block the first counter, a single Canvas view eventually needs a virtual accessibility hierarchy. Android recommends `AccessibilityNodeProvider` or `ExploreByTouchHelper` when one custom view contains multiple logical controls ([custom-view accessibility](https://developer.android.com/guide/topics/ui/accessibility/views/custom-views), [`ExploreByTouchHelper`](https://developer.android.com/reference/androidx/customview/widget/ExploreByTouchHelper)). This should derive from the same Rust element tree and hit regions rather than a second Kotlin UI model.

## Rust build and APK packaging

Build the engine as `crate-type = ["cdylib"]`; Rust defines `cdylib` specifically as a dynamic system library loaded by another language ([Rust linkage reference](https://doc.rust-lang.org/reference/linkage.html#r-link.cdylib)). Rust's Android targets are Tier 2, include `std`, and are cross-compiled with the Android NDK; `aarch64-linux-android` maps to the LP3's `arm64-v8a` ABI ([Rust Android targets](https://doc.rust-lang.org/rustc/platform-support/android.html), [Android ABIs](https://developer.android.com/ndk/guides/abis)).

For the prototype, use `cargo-ndk` directly from a Gradle task rather than introducing a Rust Gradle plugin. It configures the NDK toolchain and can emit the ABI directory structure Android expects ([cargo-ndk source and usage](https://github.com/bbqsrc/cargo-ndk)). Point the generated JNI source set at an app build directory, not checked-in `src/main/jniLibs`; Android source sets support configurable JNI-library directories ([build variants and source sets](https://developer.android.com/build/build-variants)). Make the Android packaging task depend on the Cargo task so `bun ink build`/Gradle remains the single build entry point.

Build only `arm64-v8a` for LP3 release APKs. Android warns that fat APKs containing multiple ABIs are significantly larger and provides `abiFilters` for restricting output ([Android ABIs](https://developer.android.com/ndk/guides/abis)). An `x86_64` debug variant can be added only if emulator development becomes useful.

Start with the latest installed LTS NDK supported by Rust and pin its version in Gradle/tooling. For release size, use Cargo's `strip`, LTO and `panic = "abort"` profile controls after measuring their trade-offs ([Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)); also enable Android code/resource shrinking for the Kotlin shell ([Android app-size guidance](https://developer.android.com/topic/performance/reduce-apk-size)).

## Blank launch screen

A truly absent launch screen is not available on Android 12+: the system applies a splash screen on cold and warm starts. Ink can make it visually blank and continuous with the first frame:

- `Theme.SplashScreen` as the starting theme;
- black `windowSplashScreenBackground`;
- a fully transparent drawable for `windowSplashScreenAnimatedIcon`;
- black `android:windowBackground` in the post-splash theme;
- `postSplashScreenTheme` pointing to that theme;
- `installSplashScreen()` before `super.onCreate()`;
- no artificial keep-on-screen delay.

The Android migration guide requires a splash icon drawable and post-splash theme, and documents the call order ([Android splash migration](https://developer.android.com/develop/ui/views/launch/splash-screen/migrate)). The transparent drawable is the same strategy already used by the current template in `~/Developer/light-template/plugins/withAndroidTheme.js`. Keeping the splash, window and initial Canvas background black prevents a colour flash.

## Public Sans and generated branding

Bundle the pinned Public Sans Regular TTF as `res/font/public_sans_regular.ttf` and load it once with `Resources.getFont`. Android supports bundled TTF/OTF font resources and exposes them as a `Typeface` ([font resources](https://developer.android.com/guide/topics/resources/font-resource), [fonts in XML](https://developer.android.com/develop/ui/views/text-and-emoji/fonts-in-xml)). Do not use downloadable fonts: an LP3 app should render correctly offline and deterministically on its first frame. Bundle only the weights that an app actually uses.

The canonical font should come from the Public Sans repository, pinned by version/checksum with its licence; the current upstream repository contains v2.001 but states that it is not actively maintained ([Public Sans source](https://github.com/uswds/public-sans)).

Move logo generation into the Rust Ink build tool, not the Android runtime. Preserve the template's visual rule from `~/Developer/light-template/scripts/generate-icon.js`: black square, white uppercase first letter, Public Sans Regular and vertically centred glyph metrics. Generate launcher resources from the app name during project generation/build. Prefer adaptive icon resources with separate black background and generated foreground/monochrome layers; Android adaptive icons are explicitly defined as foreground, background and optional monochrome layers ([adaptive icons](https://developer.android.com/develop/ui/compose/system/icon_design_adaptive)). The generator's image/font dependencies remain build-time dependencies and therefore do not increase the APK runtime footprint.

## Later Light integration

### Light keyboard

The keyboard has a viable existing adapter: `Lp3RawKeyboardView` extends `AbstractComposeView`, so it can live inside a normal Android view hierarchy ([Light keyboard source](https://github.com/lightphone/light-keyboard/blob/1755571b1d3353ecd3c6b68018079903ca4e389b/ui/src/main/java/com/thelightphone/lp3Keyboard/ui/Lp3KeyboardView.kt)). This is why the root should be a `FrameLayout` even though the counter initially contains only `InkView`. A future text-field focus event can show the keyboard as a bottom overlay, route coherent edit actions to Rust and hide it on blur.

Compose officially supports embedding a `ComposeView` inside a Views-based app and defines disposal strategies tied to view/lifecycle destruction ([Compose in Views](https://developer.android.com/develop/ui/compose/migrate/interoperability-apis/compose-in-views)). Keep Compose out of the core renderer and make keyboard support a capability: apps without text entry should not pull the keyboard/Compose dependency graph into their APK.

### Light SDK

Do not integrate the current SDK client into the counter prototype. Its generated manifest fixes the application/activity to `LightSdkApplication` and `LightActivity`, while `LightActivity` owns Compose content, navigation, splash lifetime and key forwarding ([manifest generator](https://github.com/vandamd/light-sdk/blob/3df3c24a21247e70ad59e1bc0393ac6d63840bc2/plugin/src/main/kotlin/com/thelightphone/plugin/ManifestGenerator.kt), [`LightActivity`](https://github.com/vandamd/light-sdk/blob/3df3c24a21247e70ad59e1bc0393ac6d63840bc2/sdk/client/src/main/kotlin/com/thelightphone/sdk/LightActivity.kt)). That is an ownership conflict, not a JNI problem.

The clean future seam is a headless Light client module that owns Binder registration, authentication, permissions and capability calls without owning the activity or UI. A Kotlin `InkLightAdapter` can then translate coarse requests/results between that module and Rust. The SDK already centralises the service binding and Parcel protocol in `LightServiceConnection`, but it is currently internal to the client module ([source](https://github.com/vandamd/light-sdk/blob/3df3c24a21247e70ad59e1bc0393ac6d63840bc2/sdk/client/src/main/kotlin/com/thelightphone/sdk/LightServiceConnection.kt)). Exposing or extracting that headless module is a prerequisite for first-class Ink integration.

## Decisions to preserve

- Rust owns app semantics, state, layout/hit testing and the display list.
- Kotlin owns Android framework objects and executes display lists.
- JNI is coarse and synchronous on the UI thread; no per-primitive calls.
- Android remains one activity, one Canvas view, plus native-backed overlays such as the keyboard.
- Public Sans and the generated first-letter icon are framework defaults.
- The system splash is visually blank, not delayed.
- The counter proves the seam; scrolling, text measurement, accessibility and SDK support deepen the same modules rather than creating parallel UI models.
