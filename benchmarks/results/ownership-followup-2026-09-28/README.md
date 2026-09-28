# Native ownership follow-up — 28 September 2026

Video progress and elapsed labels now use the existing native playback clock. Recording duration, RMS/peak and pitch have native readout components. Compiled lists produce their sparse changes during projection; map props no longer undergo a JavaScript stringify/parse round trip or a duplicate initial update.

Final Android snapshot: `experiment-20260928-221646-a486af17`. Tests use the reserved, visible Light Phone III emulator and the isolated package `com.vandam.benchmark.ownership`. Original app packages and data were not changed. The LP3 was not connected.

## List measurements

Three alternating baseline/candidate pairs, each with 200 measured iterations and 20 warm-ups. Prepared binaries and assets came from agent-tools; exact IDs and raw results are in `list-pairs.json`. These are desktop CPU scene measurements, not phone display latency.

| Mean | Before (ms) | After (ms) | Change |
| --- | ---: | ---: | ---: |
| Text/Stack single-row edit | 1.421 | 1.236 | 13.0% faster |
| Standard Row single-row edit | 1.366 | 1.134 | 17.0% faster |
| Text/Stack unrelated update | 1.185 | 1.145 | 3.4% faster |
| Standard Row unrelated update | 1.035 | 1.086 | 4.9% slower |
| Whole prepared process | 1507.77 | 1406.91 | 6.7% faster |

Not every subcase improved. The full suite and edit cases improved; unchanged-row semantics and zero-record transport checks still pass. Native scrolling, reorders, prepend/deletion, callback freshness, custom React state and bounded windows remain covered by the existing harness. The common compiled path no longer scans all projected rows a second time. A mismatched patch base deliberately uses the existing comparison path.

## Bridge review

`bindings.md` records the current native-binding fixture: 500 cells plus resize averaged 0.277 ms input-to-scene; JavaScript and transport averaged 0.154 ms and native apply/layout 0.122 ms. Hyperfine's whole-process mean was 79 ms for 200 updates plus startup, warm-up and checks. This is the explicit native-binding workload, not ordinary React.

`react-profile.md` records an instrumented ordinary React fixture. Native transfer averaged 0.051 ms, including conversion and queue submission. React render averaged 1.132 ms and commit 0.328 ms. These attribution measurements include instrumentation and do not isolate arbitrary nested JSON conversion.

The existing JSON semantics for complex props remain. The earlier paired direct primitive converter gained roughly 2% and was removed; the current measurements do not justify another conversion path. Typed network bodies remain binary. The diagnostic profiler was repaired to copy the compiler's native-list transformation alongside its other build helpers.

## Device and compatibility checks

- Template Android build, SDK/template/Reverb/Podcasts TypeScript and the existing 16 JavaScript regressions passed. Six Rust core regressions passed.
- Video elapsed text advanced from 0:01 to 0:04 with **zero ReactApply and zero ReactReady** markers in the steady sample. Pause and tap-to-seek worked. Native clock handling also covers completion, buffering and disposal; remote buffering was not induced in this test.
- Recording elapsed text advanced natively; the steady sample had **zero ReactApply and zero ReactReady** markers. Saving returned a completed recording with its final duration. Screenshots, pixel comparison and OCR verify the changing label.
- The map loaded London and its marker; a native pan changed the viewport. Marker configuration now uses explicit field comparison, preserving detection of changed coordinates, labels, order and in-place edits on render. A marker-heavy performance gain is not claimed.
- Microphone readouts mounted; starting the pitch detector reached `listening`, and its stop control was exercised. Changing-tone verification remains incomplete: the emulator crashed in `audio_forwarder_enable` inside its gRPC `injectAudio` implementation. `emulator-audio-injection.txt` retains the relevant host crash frames. The emulator was restarted visibly; the Mac's microphone was not enabled.
- An initial capture APK exposed a missing JNI static annotation. It was fixed, rebuilt and recording was rechecked on the final APK.

## Ownership after these changes

Rust owns layout, drawing, text, scrolling, images' visual lifecycle, compiled list windows, playback progress and native capture readouts. Kotlin owns Android media, capture, networking and other platform services. JavaScript owns application decisions, callbacks, subscriptions and general React semantics.

Capture hooks preserve live `state` measurements by default. Use `updates: "status"` with `RecordingDuration`, `LevelReadout` or `PitchReadout` when only the display needs the changing values. The template uses this mode. Existing custom tuners and apps can continue using live values for calculations or custom components; silently removing that behaviour would break their contract.

Remaining JS key/dependency scans, custom row execution and complex-value conversion are intentional compatibility boundaries, not proof that no further optimisation is possible. Further work should follow a measured workload. No APK-size reduction is claimed for this change.
