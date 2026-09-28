# Native playback migration — 28 September 2026

Ink now uses native playback clocks for every `usePlayer` controller. The periodic React progress mode and its `progress` option have been removed. `state.position` and `state.silenceSaved` are semantic snapshots. `getState()` reads current native values without notifying React; `seekBy()` applies relative offsets to the current native position.

Reverb uses a native controller observer instead of its 500 ms resource poll. Podcasts reads fresh state for checkpoints, source changes and saves. Its effects page refreshes only the local statistic while mounted. Both playing screens pass the native clock.

## Validation

All checks used isolated package IDs on the reserved Light Phone III emulator (`emulator-5554`). Original app installations and data were not replaced. LP3 was not connected.

| Check | Result / evidence |
| --- | --- |
| SDK, template, Reverb and Podcasts TypeScript | Passed |
| Template, Reverb and Podcasts Android builds | Passed |
| Existing binary/compiler checks plus custom-controller lifecycle regression | 16 passed; fresh reads do not publish, late events after disposal are ignored |
| Reverb progress | 0 `ReactApply` and 0 `ReactReady` during a four-second sample; elapsed label advanced (`reverb-progress.log`, screenshots) |
| Reverb pause | Screenshots two seconds apart were pixel-identical |
| Reverb next, shuffle, repeat, background/return | Track 2 displayed; native preferences stored index 1, shuffle true and queue repeat; return displayed advancing position |
| Template local playback | Playing screen displayed elapsed time and real duration |
| Podcasts fresh reads, absent periodic ticks, relative skip, negative clamp, pause/save, speed | Automated emulator assertions passed (`podcasts-final.log`) |
| Podcasts downloaded-source switch | Preserved position; repeated run with an existing download also passed |
| Podcasts 30-second checkpoint and completion | Fresh progress saved; completed episode marked finished |
| Podcasts autoplay | Downloaded next episode selected and previous episode marked finished (`podcasts-autoplay.log`) |
| Podcasts background checkpoint | Fresh next-episode position 22,491 ms saved (`podcasts-background.json`) |
| Delete the currently playing download | Real confirmation UI removed the download, changed queue source from managed file to HTTP and continued around 1:06 (`podcasts-queue-*.xml`, `podcasts-after-delete.png`) |
| Live silence-saving statistic | OCR and pixel comparison confirmed 1 → 6 seconds; only counter pixels changed (`effects-before.png`, `effects-after.png`) |

A repeated run exposed a source-switch/relative-seek edge case. Podcasts now supplies the desired start position in the queue replacement itself, instead of replacing the source and immediately issuing a relative seek against the unsettled source. Ordinary skips still use native relative seeking.

Two initial fixture assertions were corrected: the speed measurement now waits for playback to settle after a seek, and the checkpoint assertion allows the existing timer's phase instead of assuming it starts with the test step. These were test assumptions, not production fixes.

Service-disconnection recovery is implemented and compiled, but a forced isolated MediaController disconnection was not exercised. This is emulator functional coverage, not a physical-device latency benchmark or an exhaustive test of every network/service failure.

## Reproduction inputs

Agent-tool source archives and APK manifests retain the complete app copies and source hashes:

- `experiment-20260928-211724-2bb6c254`: template and Reverb builds used above.
- `experiment-20260928-212358-1ac26fee`: final Podcasts migration sequence.
- `experiment-20260928-212510-eb27e7dc`: same production code with autoplay/effects/deletion fixture.

The two temporary Podcasts routes are retained beside this report. They run inside a copy of the real Podcasts application, using its root provider and stores. Test audio was served locally at `10.0.2.2:8787`; `audio.mp3` was the template's bundled `cant_help.mp3`. The effects fixture used a 120-second mono PCM WAV, alternating one second of 440 Hz tone with three seconds of silence. Build workspaces, temporary source app copies, test installations, server and seeded emulator audio were removed after verification.
