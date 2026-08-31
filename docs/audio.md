---
title: "Audio"
description: "Play and record audio, monitor levels, and detect monophonic pitch."
---

`@ink/audio` provides playback, recording, level metering, and monophonic pitch detection. Each controller belongs to the screen where it is declared.

## Playback

Create a player with `audioPlayer()`:

```tsx
import { audioPlayer } from "@ink/audio";
import { Button } from "ink";

const player = audioPlayer({ usage: "music", playback: "detached" });

<Button onPress={() => player.play({
  src: "./assets/song.mp3",
  title: "Song",
  artist: "Artist",
})}>
  Play
</Button>
```

`usage` is `"music"` by default or `"speech"` for spoken audio. `playback` is `"attached"` by default. Attached playback stops when its screen leaves; detached playback continues through an Android media session while the app is in the background.

Sources may be:

- a bundled asset path;
- an HTTPS URL;
- the latest file saved by `audioRecorder()`.

Use `setQueue(items, startIndex)` to replace the queue without starting playback. `play()` starts or resumes the current item. The player also provides pause, toggle, stop, seek, 15-second skip, previous, next, and playback-speed actions.

The player status is `idle`, `loading`, `paused`, `playing`, `ended`, or `error`. It exposes the current item, queue index, position, duration, buffered position, and speed. Player errors distinguish source, unsupported-format, output, and unexpected failures.

## Recording

`audioRecorder()` records mono AAC audio into an app-private M4A file.

```tsx
import { audioRecorder } from "@ink/audio";

const recorder = audioRecorder();
```

- `start()` begins recording.
- `stop()` saves the recording.
- `cancel()` discards the active recording.
- `delete()` removes the saved recording.

A successful stop replaces the previous saved recording. Play it with `player.playRecording()`.

The recorder status is `idle`, `recording`, `stopping`, `ready`, or `error`. `durationMs` tracks an active recording, and `recordingDurationMs` describes the saved file.

## Microphone permission

Playback does not need microphone permission. Recording, level metering and pitch detection do.

```tsx
import { microphonePermission } from "@ink/audio";
import { Button, Text, match } from "ink";

const microphone = microphonePermission();

{match(microphone, {
  loading: () => <Text>Checking microphone</Text>,
  ready: (result) => <Text>{result.value}</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
<Button onPress={() => microphone.request()}>Allow microphone</Button>
```

## Level and pitch

Level and pitch controllers share one raw microphone capture:

```tsx
import { levelMeter, pitchDetector } from "@ink/audio";

const level = levelMeter();
const pitch = pitchDetector({ referenceHz: 440 });
```

`levelMeter()` reports normalised RMS and peak values. Its status is `idle`, `listening`, `active`, `clipping` or `error`.

`pitchDetector()` reports frequency, note, octave, cents, and confidence for monophonic input. Its status is `idle`, `listening`, `active`, or `error`. Set `referenceHz` from 400 to 480 Hz to change concert pitch from the default 440 Hz.

Start and stop either controller explicitly. Realtime analysis and recording cannot run together because both own the microphone.

## Lifecycle and packaging

Leaving a screen stops attached playback, recording, and microphone analysis. Late updates from released controllers are ignored. Detached playback may continue and reconnect when its player becomes active again.

Ink includes only the audio capabilities declared by the app. Media-session support, recording and microphone analysis do not enter an APK that only uses attached playback.
