# Playback interface — 28 September 2026

`PlayingScreen` now accepts one `playback` binding. It connects the native clock, derives transport state, invokes play/pause and seeking, and handles command failures with retry of the failed operation. There is no synchronous `player.position` getter. App logic can still read a coherent native snapshot with `await player.getState()`.

```tsx
<PlayingScreen playback={player} title={episode.title} artists={artists}
  previous={{ seconds: 10 }} next={{ seconds: 30 }} />
```

Default previous/next buttons call the binding's corresponding methods. Supplying `seconds` uses its native relative-seek method. Explicit callbacks remain available for app-specific behaviour. Async transport and bottom-action failures are handled centrally. Reverb, Podcasts and template examples were migrated; Podcasts' sleep timer, persistence and source-selection wrappers remain in the binding.

`player.replaceSource(id, src, options?)` captures position and playback intent natively, replaces the current item without clearing the queue, and preserves the current index. Optional `position` or `offset` combines a seek with replacement. `prepare: false` leaves the player stopped and paused. A stale current-item ID rejects. The native implementation uses Media3's playlist operations ([Player reference](https://developer.android.com/reference/androidx/media3/common/Player)); this is one Ink command, not a promise of gapless source replacement.

## Verification

Final build: `experiment-20260928-214001-ea9ad05e`. Source archive and APK hashes are retained by agent-tools. Tests used reserved emulator `emulator-5554` and isolated package IDs. The LP3 was not connected.

| Check | Result |
| --- | --- |
| SDK, template, Reverb and Podcasts TypeScript | Passed |
| Template, Reverb and Podcasts Android builds | Passed |
| Existing compiler/native bridge regressions | 16 passed |
| Source replacement | Emulator assertions passed for retained position, paused state, relative offset with preparation disabled, retained next queue item, and rejection of a stale item ID (`source.log`, `source.png`) |
| Podcasts migration sequence | Fresh reads, no periodic JS ticks, skip, pause/save, speed, download switching, 30-second checkpoint and episode completion passed (`podcasts-checks.log`) |
| Default skip button | Actual Podcasts UI advanced from approximately 0:05 to 0:35 with only `next={{ seconds: 30 }}` (`default-skip.png`) |
| Native progress | Four-second sample contained 0 `ReactApply` and 0 `ReactReady` messages (`native-progress.log`) |
| Autoplay | Selected the next downloaded episode and saved the previous episode as finished (`autoplay.png`) |
| Delete playing download | Native source switched back to HTTP; elapsed time advanced from 0:04 to 0:06 after deletion (`after-delete*.png`, `after-delete-queue.xml`) |
| Effects counter | While visible, OCR confirmed 1 → 6 seconds saved; pixel changes were confined to the counter (`effects-*.png`) |
| Binding failures/retry | Injected toggle rejection displayed an error; retry invoked toggle again successfully (`binding-error.png`, `binding.log`) |
| Binding units and defaults | A progress-bar tap delivered 74809.837 ms; default rewind/forward delivered -10000/+30000 ms (`binding.log`) |
| Reverb real library playback | Opened a local album track, paused, sought to 1:40 via the bar, and selected the next track through the default Next method (`reverb-playing.png`, `reverb-seek.png`, `reverb-next.png`) |

The focused temporary fixtures are retained beside this report; the full source snapshot includes the real apps and earlier migration fixtures. Audio was served locally at `10.0.2.2:8787`. The smart-speed fixture alternated one second of tone with three seconds of silence. Reverb used existing emulator music without modifying those files. Original app packages and their data were not replaced.

Temporary app copies, build workspaces, installed test packages, seeded audio and the local server were removed after verification. This is functional emulator validation, not a new LP3 latency or bundle-size claim.
