# Framework verification — 6 September 2026

This records focused checks for the implementation plan. Real-app workflows remain deferred at the user's request. No automated tests were added. The initial emulator checks below were followed by physical LP3 release benchmarks; their results and cleanup are recorded separately.

## Build and interface checks

- `cargo check --workspace --all-features` passed.
- Framework and template TypeScript checks passed. Strict template checking also passed with `skipLibCheck` disabled; the resolved declarations did not include DOM or Node globals.
- Counter and both Ink benchmark applications passed release-profile compiler checks.
- The public component props were traced through their JavaScript composition or native parse/layout/interaction paths. See [the bounded audit](public-props-audit.md).
- `ink doctor` checked Bun, Rust, Cargo NDK, Java, ADB, SDK discovery, the pinned Android NDK, Android target and APK signer successfully.

## Standalone application

A temporary app created outside the repository installed its dependencies, type-checked, compiled, installed and launched on `emulator-5554`. Its Android outputs and Gradle project cache were generated under its own `.ink` directory. Both debug and release APKs passed signature verification; the release used a temporary verification keystore and was not published or installed over the debug application.

This establishes the explicit local-SDK path, not a registry distribution. Public package names, publication, hardware launch and dependency-specific workflows remain separate work.

## Development loop

The standalone counter was incremented to two, then its component label was edited. The emulator retained two and the same process while the CLI transferred a bundle without Gradle or APK installation. A changed hook signature reset component state without reinstalling. A linked module outside the app directory changed the displayed content through bundle transfer.

Verification exposed and corrected an ADB extraction race, a redundant first graph rebuild and source maps that pointed at transformed code. The final emulator error pointed to the exact original `App.tsx:7:13`; correcting the source dismissed the dialog and recovered in the same process without reinstalling. The updated CLI installed once on initial launch.

## Scope limits

The previous template session exercised appearance, search/navigation, keyboard transitions, confirmation, wrapping fields, dynamic items and fixed-height list presentation. The latest full-capability template built, passed APK signature verification and launched through `ink dev --device emulator-5554 --once`. The final emulator pass exercised variable-row expansion, scrolling through mixed-height rows and prepending an earlier row while retaining the previous content position. The fixed-height list continued to render and increment updates. Typing with the native keyboard and submitting search opened Search Results with “Nothing found”. The final filtered Ink/Android error log was empty. End-following and image-load anchors were reviewed in source but not independently exercised in this pass.

Compilation does not establish cancellation/pause/restart stress behaviour, detached playback, real network flows, general dependency compatibility, physical input behaviour or list performance. List measurements and metadata still rebuild linearly for structural changes. Delayed JavaScript input, native keyboard composition, image-loading anchors and sustained resource use need broader interaction verification. No latency, battery or production-readiness claim follows from these checks.

## Subsequent physical LP3 benchmarks

This section records the fixtures measured at the time. The active harness is now counter-only; the [matching-counter emulator record](../benchmarks/results/matching-counter-emulator-2026-09-06/README.md) covers their later alignment. Previous physical timings do not measure those updated fixtures.

Release counter and non-virtualised 1,000-row scroll benchmarks completed on the physical LP3 for Ink, Expo and Light SDK. The Ink release check exposed an R8-stripped JNI callback; its keep rule was corrected before measurements. Separate counter checks reached 100, and scroll fixtures rendered and responded to swipes. See the [Ink results](../benchmarks/results/ink-react-lp3-2026-09-06.md) and [Expo/Light SDK results](../benchmarks/results/expo-light-sdk-lp3-2026-09-06.md). Device settings were restored and benchmark apps/APKs removed. These measurements do not cover the deferred real-app workflows or variable-height list stress scenarios.

## Subsequent conversation and keyboard checks

The template was rebuilt and installed with `ink dev --device emulator-5554 --once`. Local-fixture checks covered text/image message actions, Back, reactions, reply previews, replying from older history and sending replies. Slow upward drags across history loading and a small continued movement after loading retained the visible messages.

Keyboard checks covered removal of the dismiss row, Return inserting newlines and sending multiline text. A five-line draft stayed in a three-line composer and scrolled back to its first line. Editing line 2 retained lines 1–3 in place. Reopening the empty composer after sending, including a tap with slight movement, was exercised. Sending while scrolled above the newest message closed the keyboard, cleared the draft and showed the new message at the bottom.

Framework/template TypeScript checks, Rust workspace checks with all features and diff whitespace checks passed during these changes. No automated tests were added. These checks do not establish every frame of the keyboard transition, physical touch behaviour, long-running messaging, real services or race behaviour under delayed JavaScript. Those remain part of the joint real-app session.
