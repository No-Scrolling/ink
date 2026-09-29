# Reverb AAudio integration — 29 September 2026

**Retired:** the output integration was removed at the user’s request. Reverb retains only
the simpler connect-while-browsing optimisation (~445 ms first-song baseline). The measurements
below describe the removed experiment, not the current app.

A working **output-only** integration in Reverb. This does not migrate the complete Rust decoder
prototype into the app and does not achieve the sub-100 ms goal. There are no changes to the Ink
core or the JavaScript playback API.

## LP3 result

Physical LP3, Android 14, built-in speaker, the same three local American Football MP3s. Each run
has three fresh-process first-song selections and six subsequent selections. Album dwell is about
one second before selecting the first song, so native output has been prewarmed. Files may be in the
OS cache. Instrumented debug APKs; Rust library compiled with its release profile. No microphone
measurement: timing is JavaScript tap-handler entry to an estimated audio-output start.

| Mean | Existing Media3 output | Final native output | Change |
|---|---:|---:|---:|
| First song | 444.7 ms (421–471) | 401.0 ms (372–418) | −9.8% |
| Subsequent selections | 316.8 ms (287–341) | 253.0 ms (220–298) | −20.1% |

A preceding native-rate run measured 413.7 / 250.2 ms. These are small, sequential samples, not a
precise estimate of all-device performance. Final-run logs confirm native output for all nine
selections, one audio-start marker per selection, no playback/sink errors and zero reported AAudio
xruns at output releases. Xruns are platform underruns; they are not a comprehensive proof of
continuous or audibly perfect PCM delivery. Thermal status was 0 after measurements.

## What changed

- `android/audio`: a small app-specific Rust cdylib with a bounded 64 KiB stereo PCM ring and
  allocation-free, lock-free AAudio callback. JNI control calls run outside the callback.
- `NativeAudio.kt` and `ReverbAudioOutputProvider.kt`: Media3 output adapter, timestamp-based position,
  pause/resume, volume, flush/reuse, timeout and recoverable output-error fallback.
- `ReverbPlaybackService.kt`: supplies the output provider and warms it using Media3's actual audio
  session ID. Existing media session, audio focus, queue, decoder and file-access implementations remain.
- `android/build.gradle.kts`: builds/packages the Rust library for Reverb only.

Native output is selected for **44.1 kHz stereo 16-bit PCM on the built-in speaker**. Other formats,
channels and routes retain Media3's AudioTrack output. No additional resampler is introduced. The
silent warm stream closes after 30 seconds without an output owner. Paused active output suspends
after 30 seconds. This trades a bounded period of audio-resource use for faster recent interactions;
it does not establish the same latency after a long idle or measure battery impact.

## Why this is slower than the 37 ms prototype

The prototype opened/decoded a short MP3 prefix directly in Rust and published it to an already
running 48 kHz AAudio stream. Reverb still runs Media3's extraction, decoding and playback scheduling.
It also preserves the real audio session ID and original 44.1 kHz rate. LP3 logs explicitly report
`MMAP not used because sessionId specified` and downgrade requested performance mode 12 to 10.
Consequently this integration does not use the prototype's output configuration.

Forcing 48 kHz through Media3's Sonic processor measured 368.3 / 177.3 ms, but introduced additional
linear sample-rate conversion into music playback; it was removed. Reducing Media3 startup buffering
measured 359.7 / 193.7 ms with that resampled variant and gave no convincing further benefit; it was
also removed. Failed early session/start-state experiments were corrected before accepted runs.

Getting closer to the prototype calls for a native streaming/decoder core with established codec
libraries, high-quality resampling where required, bounded buffering and proper seek/gapless support.
Android should still own media sessions, audio focus, notifications and service lifecycle. The app
must open permitted content URIs/FDs; the prototype's shell-accessible filesystem paths are not a
production permission strategy. Format coverage and route behaviour need testing before replacing
Media3 wholesale.

## Validation

- Debug Android build, TypeScript check and Rust Clippy with warnings denied.
- Release-optimised Android build and emulator/LP3 launch/playback using the local **development signing
  key**. Reverb has no production signing configuration; the temporary test signing configuration was
  restored immediately after building.
- Emulator MP3 playback, pause, seek while paused, resume after the 30-second suspend timeout, and
  automatic next-track transition.
- Native 44.1 kHz stereo WAV → fallback 48 kHz mono WAV automatic transition and end-of-queue.
- 96 kHz FLAC playback through fallback. Generated emulator-only fixtures removed afterwards.
- LP3 repeated MP3 selections, background playback, pause, seek while paused, resume and media-button next/pause. App data was preserved.
- Saved screenshots verified using `scripts/agent-tools image --ocr`; no renderer changes.

Bluetooth/wired route changes, APE/AAC/Opus/Vorbis, acoustic gaplessness and long-duration battery/
reliability testing were not comprehensively exercised. This is a limited integration, not evidence
that an entire replacement playback engine is ready.

## Evidence

`summary.json`, per-variant JSON and filtered logs retain accepted timings; `measurement.json`
identifies the instrumented APK. Full logs and temporary instrumentation are in
`.agent-tools/audio-integration/`. The final app source removes the timing hooks. The named final
run predates diagnostic/FFI visibility cleanup, the unknown-route guard and using the public
ExoPlayer provider setter instead of a renderer override, not a different buffering or decoding strategy.
