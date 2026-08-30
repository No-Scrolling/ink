# Expo audio capabilities relevant to Ink

Research date: 30 August 2026. Sources are limited to Expo's official documentation and first-party source.

## Summary

Pitch detection is not the only useful microphone accessory, but Expo does not organise audio around ready-made analysers. Its current `expo-audio` package provides playback, playlists, encoded recording, recording level metering, playback samples, and a general raw-PCM microphone stream. Applications perform any higher-level analysis themselves. The published API reference contains no pitch, note, FFT, spectrum, onset, beat, voice-activity or silence detector; this is an inference from the complete first-party API surface rather than an explicit Expo product statement ([Expo audio API](https://docs.expo.dev/versions/latest/sdk/audio/), [Expo audio source](https://github.com/expo/expo/tree/main/packages/expo-audio)).

For Ink, Expo is useful evidence that live PCM is the general extension point. It is not evidence that PCM should cross Ink's TypeScript boundary. On the LP3, purpose-built native analysers can provide a much smaller and safer interface.

## What Expo provides

### Playback

`expo-audio` has lifecycle-managed and manually managed players. Sources can be bundled assets, local files, or remote URLs with optional HTTP headers. A player exposes play, pause, source replacement, seeking, looping, volume, mute, playback rate with pitch correction, loading/buffering state, duration, live-stream state, errors, and periodic status updates. Hook-owned players are released automatically; directly created players must be released by the application ([player creation and lifecycle](https://docs.expo.dev/versions/latest/sdk/audio/#useaudioplayersource-options), [player and source API](https://docs.expo.dev/versions/latest/sdk/audio/#audioplayer)).

The package also has a gapless playlist controller with add, insert, remove, clear, next, previous, index selection, seeking, loop modes, volume, rate, and track-change/status events. The current documented playlist API does not include shuffle ([playlist hook](https://docs.expo.dev/versions/latest/sdk/audio/#useaudioplaylistoptions), [playlist API](https://docs.expo.dev/versions/latest/sdk/audio/#audioplaylist)).

Expo supports preloading and `downloadFirst`. Preloading buffers a source for later use and has explicit clearing APIs. `downloadFirst` places the complete source in temporary storage on Android/iOS, or device memory on web; the system can purge it. This is transient preparation, not a durable user-managed download library ([preloading](https://docs.expo.dev/versions/latest/sdk/audio/#audiopreloadsource-options), [`AudioPlayerOptions`](https://docs.expo.dev/versions/latest/sdk/audio/#audioplayeroptions)).

On Android, Expo's first-party build includes Media3 ExoPlayer plus session, UI, DASH, HLS, SmoothStreaming, data-source and OkHttp modules. Playback formats otherwise follow the formats supported by the native platform player ([Android build dependencies](https://github.com/expo/expo/blob/main/packages/expo-audio/android/build.gradle), [Expo format guidance](https://docs.expo.dev/versions/latest/sdk/audio/)).

### Background controls and audio policy

Background playback is build-time opt-in. On Android it adds a media playback foreground service and permissions; sustained background playback also requires activating lock-screen controls. One player can own those controls at a time, with title, artist, album, artwork, live-stream handling, and optional seek buttons. The same integration supplies a media notification and lock-screen playback controls ([background playback](https://docs.expo.dev/versions/latest/sdk/audio/#playing-audio-in-the-background), [lock-screen API](https://docs.expo.dev/versions/latest/sdk/audio/#setactiveforlockscreenactive-metadata-options)).

Global audio mode controls whether the app mixes, ducks other audio, or requests exclusive focus; whether playback remains active in the background; silent-mode behaviour; recording; and earpiece routing. Expo documents that Android's `mixWithOthers` requests no audio focus and therefore does not receive focus-loss callbacks such as phone-call interruptions. It also documents automatic stopping after headphone or Bluetooth disconnection ([audio mode](https://docs.expo.dev/versions/latest/sdk/audio/#audiomode), [interruption modes](https://docs.expo.dev/versions/latest/sdk/audio/#interruptionmode), [headphone behaviour](https://docs.expo.dev/versions/latest/sdk/audio/)).

### Encoded recording

The recorder supports prepare, record, pause, stop, duration-limited recording, state updates, optional level metering, and input-device enumeration/selection. Options cover channel count, sample rate, bitrate, platform encoder/container settings, microphone source on Android, and output directory. Expo supplies high- and low-quality presets; the high-quality Android preset is stereo 44.1 kHz, 128 kbit/s AAC in MPEG-4. Recordings default to cache storage and can instead use the app's document directory ([recording API](https://docs.expo.dev/versions/latest/sdk/audio/#audiorecorder), [recording options and state](https://docs.expo.dev/versions/latest/sdk/audio/#recordingoptions), [recording storage](https://docs.expo.dev/versions/latest/sdk/audio/#recording-sounds)).

Microphone permission has separate get and request calls. Expo can also opt into background recording; on Android that adds microphone foreground-service and notification permissions and shows a persistent recording notification with a stop action ([recording permission](https://docs.expo.dev/versions/latest/sdk/audio/#audiorequestrecordingpermissionsasync), [background recording](https://docs.expo.dev/versions/latest/sdk/audio/#recording-audio-in-the-background)).

### Analysis primitives

Expo exposes three different levels of information:

- Recorder metering supplies a current audio-level value when enabled. It is suitable for an input level indicator or clipping feedback, but it is not raw audio ([`RecorderState`](https://docs.expo.dev/versions/latest/sdk/audio/#recorderstate)).
- Player sampling supplies normalised PCM frames per channel from media already being played. Expo describes this as suitable for visualisation, analysis, or processing; Android requires microphone permission, and support is platform-dependent ([player sample listener](https://docs.expo.dev/versions/latest/sdk/audio/#useaudiosamplelistenerplayer-listener), [`AudioSample`](https://docs.expo.dev/versions/latest/sdk/audio/#audiosample)).
- `AudioStream` captures raw microphone PCM in real time. Callers choose mono/stereo, desired sample rate, and `float32` or `int16`; buffers contain an `ArrayBuffer`, actual sample rate, channels, and timestamp ([stream hook and lifecycle](https://docs.expo.dev/versions/latest/sdk/audio/#useaudiostreamoptions), [stream buffer and options](https://docs.expo.dev/versions/latest/sdk/audio/#audiostreambuffer)).

The last primitive makes tuners possible, but Expo leaves pitch detection and every other signal-processing algorithm to the application.

### Web differences

Expo maps recording to the browser `MediaRecorder` API. It warns that Chrome WebM files can omit duration metadata, browser encoding options are inconsistent, and microphone access requires a secure origin. Player downloading uses browser memory and CORS applies; audio sources and PCM sampling can also require suitable CORS headers ([web notes](https://docs.expo.dev/versions/latest/sdk/audio/#notes-on-web-usage), [`AudioPlayerOptions`](https://docs.expo.dev/versions/latest/sdk/audio/#audioplayeroptions), [`AudioSource`](https://docs.expo.dev/versions/latest/sdk/audio/#audiosource)). These differences are irrelevant to Android-only Ink but illustrate the cost of exposing a cross-platform raw-media abstraction.

## The deprecated `expo-av` surface

`expo-av` previously combined audio and video. Its audio API had imperative `Audio.Sound` and `Audio.Recording` objects, playback status callbacks, audio modes, samples, metering and configurable recording. Expo deprecated both the video and audio APIs, stopped patching the package, and directed applications to `expo-video` and `expo-audio`; the official SDK 54 documentation stated that it would be removed in SDK 55 ([deprecated AV documentation](https://docs.expo.dev/versions/v54.0.0/sdk/av/), [deprecated audio documentation](https://docs.expo.dev/versions/v54.0.0/sdk/audio-av/)). Ink should compare against `expo-audio`, not reproduce `expo-av`'s broad imperative object.

## Implications for `@ink/audio`

The useful public capabilities for Ink divide into three layers:

1. **Core:** playback, queue, recording, permissions, background ownership, audio focus, routes, metadata, controls, and lifecycle.
2. **Small analysers:** `levelMeter()` and `pitchDetector()` are both justified. Level is useful for recorder feedback, microphone checks, clipping indication and voice-note UX; pitch serves tuners and musical tools.
3. **Deferred specialist analysis:** waveform summaries, voice activity/silence, spectrum/FFT, onset/beat, and loudness are plausible but should wait for a real application. Voice activity is more likely to be an internal recorder policy than a first-class controller.

Do not expose Expo-style PCM `ArrayBuffer`s to Ink applications initially. It would create high-frequency bridge traffic, make app code responsible for realtime buffering and lifecycle, and conflict with Ink's declarative TypeScript-to-Rust model. Build one hidden native PCM capture seam and run processors in Rust:

```ts
const level = levelMeter();
// "idle" | "listening" | "active" | "clipping" | "error"
// level.rms, level.peak

const tuner = pitchDetector({ referenceHz: 440 });
// frequencyHz, note, octave, cents, confidence
```

This is broader than pitch without becoming a general-purpose digital audio workstation API. `levelMeter()` and `pitchDetector()` can share the same capability-gated `AudioRecord` capture engine, microphone permission, exclusive microphone ownership, cancellation and screen lifecycle. No encoded recorder or Media3 playback dependency is needed when an application only uses analysis.
