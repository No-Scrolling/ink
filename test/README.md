# Tests

Run host correctness checks from the repository root with the installed Rust toolchain, Bun and workspace dependencies:

```sh
bun run test
bun run test compiler
bun run test runtime scene --profile release
bun run test scene --filter prepending
bun run test android --serial emulator-5554
bun run test android-app --serial emulator-5554
bun run test visual --serial emulator-5554
```

The default suites are `unit`, `compiler`, `sdk`, `runtime`, `scene` and `audio`. `--profile` selects `ink-dev` (default) or `release`; `--filter` selects a Rust test-name substring or Bun test-name regular expression. A host suite selecting no tests fails. Device suites are opt-in and require an explicit serial. Run `bun run test --help` for available options.

Android suites also work with a headless emulator. Start `emulator -avd Light_Phone_III -writable-system -no-window`, wait for boot, then run the same commands with `--serial`. ADB still supplies input and captures rendered screenshots. The test runners leave emulator startup and shutdown to the caller.

| Suite | Production path and limits |
| --- | --- |
| `unit` | Existing Rust module checks, physically stored in `test/unit`. Private access remains attached to the owning crate. |
| `compiler` | Real JSX, routing and icon transforms; emitted output uses Ink's genuine helpers. Bun execution does not establish QuickJS behaviour. |
| `sdk` | Portable SDK data/validation contracts executed in Bun. Native IO and Android lifecycle require the integration suites. |
| `runtime` | Fixtures compiled by Ink's compiler, executed by `AppRuntime` in real QuickJS, including lazy split-web loading and direct runtime transport/lifetime checks. |
| `scene` | Compiled fixtures through QuickJS, React commits, `ReactTree`, layout and interaction, including navigation and native-view lifetime. No Vulkan, Android activity lifetime or physical display measurements. |
| `audio` | Production level/pitch processors with analytically known signal inputs. No microphone or playback-device claims. |
| `android` | Selected production Kotlin adapters on Android, using a separate fixture APK. See `android/README.md` for prerequisites and cleanup. |
| `android-app` | Public store/network SDKs through the real compiled Android app, QuickJS, Rust/JNI and Kotlin. Uses SQLite, a local HTTP server and a process restart. See `android/app.md`. |
| `visual` | Saved pixels from the real Android renderer, compared with explicitly reviewed references. Requires prior approval; see `visual/README.md`. |

Native fixtures are compiled afresh before their selected suites. Rust profiles select native optimisation settings; fixture preparation uses production JavaScript bundling, compiling the web fixture with `INK_SPLIT_WEB=0` and `1`, and the other host fixtures with `0`. Device suites build development APKs. The runner continues independent suites and Cargo targets after failures and returns a non-zero status if preparation or a suite fails.

`bun test` runs only the TypeScript suites; use `bun run test` to include Rust and QuickJS/scene integration. Direct Cargo integration runs require prepared fixtures:

```sh
bun test/native/prepare.ts runtime scene --profile ink-dev
cargo test --profile ink-dev -p ink-test --test runtime
```

Run logs, selected commands, source hashes and tracked changes go into ignored `.test-output/`. Fixture bundles remain in ignored `.ink/` directories; Cargo output remains in ignored `target/`. A dirty-tree hash inventory identifies inputs but is not a recoverable source snapshot: commit the source before making reproducibility claims. Generated evidence is not committed automatically.

Tests remove temporary compiler directories. The Android runner uninstalls its fixture APK and removes its device lock in `finally`. Build caches, fixture bundles and run logs remain ignored for reuse and diagnosis; they are not deleted after each run.

Each test must protect an observable contract, catch a realistic defect and use independently derived expectations. Suite notes record coverage and limitations. Existing checks were reviewed during migration; see `unit/README.md` and `compiler/README.md`.

These are correctness suites. They do not infer speed from elapsed test duration, and they do not add benchmark thresholds or unreviewed screenshot references.

Run `bun run bench` for the separate release-mode QuickJS/scene performance suite. It measures bound/React updates, list scaling and realistic rows, navigation/startup, conversation updates and typing under load, native playback clocks and idle behaviour. See [performance/README.md](performance/README.md) for measurement boundaries, contracts, saved baselines and alternating comparisons.

After approving visual references, select every suite explicitly:

```sh
bun run test unit compiler sdk runtime scene audio android android-app visual --serial emulator-5554
```
