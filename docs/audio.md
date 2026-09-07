---
title: "Audio"
description: "Play audio, record files and analyse microphone input."
---

Use `@ink/audio` to play audio, manage queues and connect to native media controls. Import `@ink/audio/microphone` for analysis or `@ink/audio/recording` to record files.

Ink handles decoding, buffering, audio focus and routing natively. Your app sends commands and observes playback state.

```tsx
import { Button, Screen, Text, useAction } from "ink";
import { usePlayer } from "@ink/audio";

export function NowPlaying() {
  const player = usePlayer({ session: "main", mode: "detached" });
  const toggle = useAction(() => player.toggle());
  return (
    <Screen title="Now playing">
      <Text>{player.state.current?.title ?? "Nothing playing"}</Text>
      <Button onPress={() => toggle.run()} disabled={!player.state.ready}>
        {player.state.playing ? "Pause" : "Play"}
      </Button>
      {toggle.status === "error" && <Text>{toggle.error.message}</Text>}
    </Screen>
  );
}
```

`session` defaults to `main`. Names contain 1–64 letters, numbers, dots, underscores or hyphens; up to eight sessions can be active. Attachments with the same name share playback and must use the same mode and usage. Different names retain independent queues. Playback is exclusive: starting a session pauses other sessions rather than mixing audio. `usePlayer` attaches after commit and releases its attachment when hidden. Wait for `state.ready` before issuing commands or inspecting a restored queue. Reconnecting observes the live session; mounting the screen does not replace its queue.

## Choose a playback lifetime

An **attached** player is useful for a short voice-note preview. Releasing its last owner stops it. A **detached** session supports music, podcasts and audiobooks: releasing a screen attachment leaves playback under a native media service. `stop()` explicitly stops playback; closing a screen does not.

Android can stop a detached session. Ink saves its queue, index, speed and progress, then restores it paused when you reconnect. Queue changes, seeks and pauses save immediately; progress saves every five seconds. `stop()` clears the queue and saved state. Deleted files and unavailable URLs report playback errors. Background workers cannot attach audio controllers.

## Queue and state

`setQueue(items, { startIndex })`, `play`, `pause`, `seek`, `next`, `previous` and `stop` return promises. Queue items have stable IDs, a `src` string naming bundled audio or a supported URL, title and optional artist/artwork metadata. Call `setQueue` when you want to replace the queue.

Snapshots include readiness, current item, position, duration, buffering, playing and error. Position is sampled at a useful display rate; smooth native progress and seeking do not need frame-rate JavaScript events. Errors distinguish source, unsupported media and output failures. Skipping a broken item is an app policy, not an automatic consequence of any error.

Native media sessions coordinate hardware keys, lock-screen controls and focus interruptions. State reflects changes made outside the app. Commands issued before readiness reject rather than silently overwriting restored state.

## Recording and analysis

`@ink/audio/microphone` exports `useLevelMeter`, `usePitchDetector` and `PitchIndicator`. `@ink/audio/recording` exports `useRecorder`. Both entry points export `microphone` for permission checks. Request access when the user opens a feature that needs it using `microphone.requestPermission()`; `getPermission()` reads the existing grant. Permission results are `granted`, `denied` or `blocked`.

Each capture hook returns `ready`, `state` and promise-returning commands. Wait for readiness before enabling controls. Hook cleanup releases the native attachment; capture is foreground work, not a background recording service. Active pitch and level analysers pause when the app leaves the foreground and resume when it returns. Calling `stop()` or disposing the hook cancels this automatic resumption. Recordings stop on backgrounding and do not restart automatically.

| Hook | Commands | State |
| --- | --- | --- |
| `useRecorder()` | `start()`, `stop()`, `cancel()`, `delete()` | Status, duration in milliseconds, error and the saved recording when available. |
| `useLevelMeter()` | `start()`, `stop()` | Status, RMS, peak and error. These are signal levels, not calibrated sound-pressure measurements. |
| `usePitchDetector({ referenceHz })` | `start()`, `stop()` | Status, frequency in Hz, note, octave, cents, confidence and error. Reference defaults to 440 Hz. |

`PitchIndicator` from `@ink/audio/microphone` displays a 25-tick tuning gauge. Pass the detector’s `state.cents` while active, or `null` when no note is detected. The taller centre tick marks the target; readings within ±5 cents settle there. The scale spans −50 to +50 cents, with flat notes on the left and sharp notes on the right. It follows Ink’s colour scheme and does not handle touch input.

Recorder status is idle, recording, stopping, ready or error. A completed recording is a [FileRef](/files#one-file-representation) with `id`, `src`, `name`, `mimeType`, `size` and `duration` in milliseconds. Its source can be played by `usePlayer` or read with `fetch`. Read completion from state rather than treating `stop()` as a returned recording. Cancel discards unfinished capture; delete removes the current saved recording.

Completing another recording retains earlier accepted recordings. Persist their IDs alongside app data and reopen them with `files.open(id)`. They survive screen disposal and app restarts until `files.remove(id)`, recorder deletion of the current recording, or app-data removal. The recorder exposes its most recent recording; the app owns its library and retention policy.

The microphone entry point includes analysis without recording, managed files or image decoding. The recording entry point includes file storage without the pitch and level analysis engine. Apps using both share the audio adapter, which prevents recording and analysis from taking the microphone at the same time. Playback-only apps omit both input engines and microphone permission. See the [build contract](/build-contracts#product-contract).

## Providers and offline media

A podcast enclosure or owned audio file can use this player directly. A music service may require a provider SDK, remote-control session, DRM or a dedicated native engine. Its package exposes domain commands and snapshots appropriate to that provider; generic audio does not grant catalogue or offline playback rights.

[Downloads](/downloads) supplies durable managed files. Resolve a downloaded ID to current content before playback; deletion and expired provider access remain possible.
