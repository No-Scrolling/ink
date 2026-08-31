# Audio

Import audio capabilities from `@ink/audio`. The package is compile-time only: Ink includes each native capability only when an application declares it.

## Playback

`audioPlayer()` creates a screen-scoped player. `usage` selects music or speech audio attributes; `playback` is `"attached"` by default or `"detached"` for playback that continues through an Android media session while the app is in the background.

```tsx
const player = audioPlayer({ usage: "music", playback: "detached" });

<Button onPress={() => player.play({
  src: "./assets/song.mp3",
  title: "Song",
  artist: "Artist",
})}>
  Play
</Button>
```

Sources may be relative bundled assets, HTTPS URLs or recordings produced by Ink. `setQueue(items, startIndex)` replaces the queue without starting it; call `play()` afterwards. `previous()` and `next()` move between items, while seek, skip and playback-speed operations affect the current item. Literal queue indices and speeds are checked by the compiler.

The player status is `idle`, `loading`, `paused`, `playing`, `ended` or `error`. The error branch exposes `error.kind`, `error.message` and `error.retryable`. Source, unsupported-format and audio-output failures remain distinct.

## Recording

`audioRecorder()` records mono AAC-LC audio into an app-private M4A file. `start()`, `stop()` and `cancel()` control the current recording. A successful stop replaces the previous saved recording; `delete()` removes it. The saved source can be played directly with `player.playRecording()`.

The recorder reports `idle`, `recording`, `stopping`, `ready` or `error`. `durationMs` is the live duration and `recordingDurationMs` describes the saved file.

## Microphone analysis

`microphonePermission()` exposes the usual loading, ready and error resource states plus `request()`. Level and pitch controllers share one raw microphone capture:

```tsx
const level = levelMeter();
const pitch = pitchDetector({ referenceHz: 440 });
```

`levelMeter()` reports normalised RMS and peak values. `pitchDetector()` reports frequency, note, octave, cents and confidence for monophonic input. Either controller is started and stopped explicitly. Recording and realtime analysis are mutually exclusive because they own the same microphone.

Players and recorders are activated only while their screen is visible. Leaving a screen releases attached playback, stops active recording and capture, and ignores late controller updates. Detached playback may continue through its Android media session and reconnect when a player becomes active again. An application may declare players on separate routes, but Ink rejects layouts that could activate two players or two recorders simultaneously.
