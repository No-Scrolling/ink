# Framework verification — 6 September 2026

This records focused checks for the implementation plan. Real-app workflows and physical Light Phone III measurements are deferred at the user's request. No automated tests were added; no physical-device settings or applications were changed.

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
