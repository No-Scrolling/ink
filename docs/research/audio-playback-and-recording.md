# Audio playback and recording for Ink

Research date: 30 August 2026. Sources are limited to official Android documentation and source, the Light SDK source at commit [`3df3c24`](https://github.com/lightphone/light-sdk/tree/3df3c24a21247e70ad59e1bc0393ac6d63840bc2), and measurements of locally cached official AndroidX artefacts.

## Recommendation

Create a conditional `@ink/audio` extension with two framework-owned controllers:

- one process-wide audio player, backed by Media3 `ExoPlayer`;
- one foreground-only recorder, backed by the platform `MediaRecorder`.

Do not expose either as an `AsyncResource`. A resource is a lazy read which eventually becomes ready or fails; a player and recorder are long-lived native objects with commands and continuing state changes. Add an internal native-controller seam which combines an observable snapshot with command methods. The public interface can remain small and fully typed while the controller owns lifecycle, audio focus, route changes, permissions and native errors.

Use the Light SDK's attached/detached ownership model, but do not depend on the entire Light SDK client artefact. Its audio implementation already validates the architecture: attached playback owns an in-process `ExoPlayer`, while detached playback uses a `MediaController` to control an `ExoPlayer` in a `MediaSessionService`; the platform `MediaSession` is the LightOS now-playing boundary ([Light SDK detached-audio decision](https://github.com/lightphone/light-sdk/blob/3df3c24a21247e70ad59e1bc0393ac6d63840bc2/docs/design_decisions/detached_audio.md), [player source](https://github.com/lightphone/light-sdk/blob/3df3c24a21247e70ad59e1bc0393ac6d63840bc2/sdk/client/src/main/kotlin/com/thelightphone/sdk/audio/LightAudioPlayer.kt)). The current Light SDK client module also brings Compose, Ktor, Room, WorkManager and other unrelated dependencies, so using it wholesale would weaken Ink's pay-only-for-what-you-use design ([Light SDK client dependencies](https://github.com/lightphone/light-sdk/blob/3df3c24a21247e70ad59e1bc0393ac6d63840bc2/sdk/client/build.gradle.kts)). If Light SDK later publishes a focused audio artefact, Ink can replace its hidden Android adapter without changing application code.

Media3 is the right playback implementation despite being larger than `MediaPlayer`. Android explicitly recommends Media3; `MediaPlayer` has a fragile state machine, and calling operations in the wrong state produces exceptions or undefined behaviour ([MediaPlayer state and resources](https://developer.android.com/media/platform/mediaplayer/state-resources)). `ExoPlayer` adds streaming protocols, buffering and a consistent `Player` interface, while still using Android's platform decoders by default rather than bundling codecs ([Media3 ExoPlayer](https://developer.android.com/media/media3/exoplayer), [supported formats](https://developer.android.com/media/media3/exoplayer/supported-formats)). Ink's Spotify-like use case needs reliable remote playback, queues, seeking, metadata, focus and LightOS controls; rebuilding those around `MediaPlayer` would trade APK bytes for substantial framework complexity.

Use stable Media3 `1.11.0`, released on 5 August 2026, and pin every Media3 module to the same version ([Media3 releases](https://developer.android.com/jetpack/androidx/releases/media3)). Do not include `media3-ui`, DASH, HLS, software decoder or download modules until an application needs them.

## Proposed public interface

The names are deliberately nouns rather than React-style hooks: Ink owns the objects and their lifetime.

```ts
// @ink/audio
import type { AsyncResource } from "ink";

export type AudioUsage = "music" | "speech";
export type AudioPlayback = "attached" | "detached";

export interface AudioItem {
  readonly id: string;
  readonly src: string;
  readonly title: string;
  readonly artist?: string;
  readonly album?: string;
  readonly artwork?: string;
}

export type AudioPlayerErrorKind =
  | "source"
  | "unsupported"
  | "output"
  | "unexpected";

export interface AudioPlayerError {
  readonly kind: AudioPlayerErrorKind;
  readonly message: string;
  readonly retryable: boolean;
}

type AudioPlayerActiveState<Status extends string> = {
  readonly status: Status;
  readonly item: AudioItem;
  readonly index: number;
  readonly positionMs: number;
  readonly durationMs: number;
  readonly bufferedMs: number;
  readonly speed: number;
};

export type AudioPlayer = AudioPlayerCommands &
  (
    | { readonly status: "idle" }
    | AudioPlayerActiveState<"loading" | "paused" | "playing" | "ended">
    | {
        readonly status: "error";
        readonly error: AudioPlayerError;
      }
  );

export interface AudioPlayerCommands {
  setQueue(items: ReadonlyArray<AudioItem>, startIndex?: number): void;
  play(item?: AudioItem): void;
  pause(): void;
  toggle(): void;
  stop(): void;
  seekTo(positionMs: number): void;
  skipBack(): void;
  skipForward(): void;
  previous(): void;
  next(): void;
  setSpeed(speed: number): void;
}

export declare function audioPlayer(options?: {
  readonly usage?: AudioUsage;
  readonly playback?: AudioPlayback;
}): AudioPlayer;

export type MicrophonePermission = AsyncResource<
  "granted" | "denied" | "blocked" | "unknown"
> & {
  request(): void;
};

export declare function microphonePermission(): MicrophonePermission;
export declare function audioRecorder(): AudioRecorder;
```

The real declaration should spell out each discriminated-union branch. Active player branches expose `item`, `index`, `positionMs`, `durationMs`, `speed` and `bufferedMs`; the error branch exposes only an SDK-owned structured error. `play(item)` is the common one-tap path and replaces the queue with that item. `setQueue()` exists for albums and playlists. A queue error should stop, not silently skip: skipping is application policy and an invalid queue could otherwise loop, matching the Light SDK's decision ([Light SDK playback errors](https://github.com/lightphone/light-sdk/blob/3df3c24a21247e70ad59e1bc0393ac6d63840bc2/docs/design_decisions/detached_audio.md#playback-errors)).

Sources are:

- a compile-time bundled asset such as `./assets/song.mp3`;
- an HTTPS URL streamed by ExoPlayer;
- an opaque recording source returned by Ink, such as `ink://audio/recordings/<id>`.

HTTP audio should not pass through `@ink/network`: playback needs streaming, byte ranges, buffering and seeking rather than a bounded whole-response download. Release builds should reject cleartext `http://` sources, just as Ink networking already does.

Recording should use another discriminated union:

```ts
type AudioRecorder = AudioRecorderCommands &
  (
    | { readonly status: "idle" }
    | { readonly status: "recording"; readonly durationMs: number }
    | { readonly status: "stopping" }
    | {
        readonly status: "ready";
        readonly recording: AudioRecording;
      }
    | { readonly status: "error"; readonly error: AudioRecorderError }
  );

interface AudioRecorderError {
  readonly kind:
    | "permission-denied"
    | "unavailable"
    | "output"
    | "unexpected";
  readonly message: string;
  readonly retryable: boolean;
}

interface AudioRecorderCommands {
  start(): void;
  stop(): void;
  cancel(): void;
  delete(recording: AudioRecording): void;
}

interface AudioRecording {
  readonly id: string;
  readonly src: string;
  readonly durationMs: number;
}
```

`microphonePermission()` should follow Ink's existing explicit permission flow with typed `"granted" | "denied" | "blocked" | "unknown"` states and `request()`. Merely declaring a recorder causes the compiler to add `android.permission.RECORD_AUDIO`; it must not prompt until application code calls `request()`. Android classifies this as a dangerous runtime permission ([`RECORD_AUDIO`](https://developer.android.com/reference/android/Manifest.permission.html#RECORD_AUDIO), [recording permission guide](https://developer.android.com/media/platform/mediarecorder#requesting-permission)).

## Runtime design

### A native controller seam

The existing Ink native bridge is request/completion based: resources read once, actions complete once, and cancellation terminates work. Audio instead needs unsolicited state updates for buffering, playing, completion, queue transitions, errors and recording duration. Add an internal controller abstraction with:

1. a compile-time controller ID and owner;
2. a native snapshot stored by the Rust engine;
3. command requests carrying the controller ID;
4. a Kotlin-to-Rust `nativeUpdateController` event which replaces the snapshot and invalidates the visible scene;
5. explicit activation, detachment and destruction.

This should be a deep internal module, not a generic TypeScript API. Player and recorder declarations are the only public concepts. Commands remain cold user actions; observing controller state never starts playback or recording.

Player progress has no callback in the `Player` interface, so Android recommends querying position at suitable intervals ([Media3 `Player` interface](https://developer.android.com/media/media3/session/player)). Poll at 250 ms only while playing and only deliver UI updates while a screen observes the player. Media3 state transitions and failures should otherwise arrive through `Player.Listener` ([player events](https://developer.android.com/media/media3/exoplayer/listening-to-player-events)). This avoids a permanent render loop.

### Playback ownership

- **Attached** is the default. The declaring screen owns an in-process player; leaving that screen stops and releases it. It needs `media3-common` and `media3-exoplayer`, but no service or foreground-service permissions.
- **Detached** is explicit and application-owned. It controls a single `ExoPlayer` hosted by a same-process `MediaSessionService`, so playback, queue and position can survive screen navigation or the activity leaving the foreground. The compiler conditionally adds `media3-session`, `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_MEDIA_PLAYBACK`, and a `mediaPlayback` service declaration. Android requires exactly this service model for background playback ([background playback](https://developer.android.com/media/media3/session/background-playback)).

Only one player and one media session should exist per application. Android notes that most media applications do not need multiple sessions, and multiple sessions also present as multiple media apps to external controllers ([`MediaSessionService`](https://developer.android.com/reference/androidx/media3/session/MediaSessionService)). Enforcing this in the compiler prevents competing players, duplicate focus ownership and unnecessary memory.

The detached service should follow the Light SDK's proven lifetime rule: active playback keeps it alive; once playback is not playing and no Ink screen holds the controller, stop and release it after 15 minutes ([Light SDK service](https://github.com/lightphone/light-sdk/blob/3df3c24a21247e70ad59e1bc0393ac6d63840bc2/sdk/client/src/main/kotlin/com/thelightphone/sdk/audio/LightAudioService.kt)). Reconnecting must first adopt an existing live queue instead of replacing it.

Set appropriate audio attributes and let ExoPlayer manage audio focus by passing `handleAudioFocus = true`; Android recommends this path ([audio focus](https://developer.android.com/media/optimize/audio-focus)). Because Ink targets API 36, attached playback can request focus only while the app is topmost, while detached playback qualifies through its foreground service. Also enable `setHandleAudioBecomingNoisy(true)`: the Media3 default is `false`, and enabling it pauses when a headset route is disconnected rather than unexpectedly playing through the speaker ([`ExoPlayer.Builder`](https://developer.android.com/reference/androidx/media3/exoplayer/ExoPlayer.Builder#setHandleAudioBecomingNoisy(boolean))).

Publishing the queue and metadata through `MediaSession` gives LightOS, Bluetooth and headset buttons a standard control surface. Media3 automatically keeps session state in sync with its player ([media-session controls](https://developer.android.com/media/media3/session/control-playback)).

### Recording ownership

Use `MediaRecorder`, not Media3 or `AudioRecord`, for encoded files. Configure processed `MIC` input, MPEG-4, AAC-LC, mono, 48 kHz and 96 kbit/s. This produces `.m4a` files which the same player can consume at roughly 0.7 MB per minute; AAC-LC encoding and MPEG-4/M4A are mandatory platform capabilities on Android 14 ([Android supported formats](https://developer.android.com/media/platform/supported-formats)). Raw PCM capture and realtime analysis are different features and should not enlarge the initial recorder API.

Ink owns filenames and exposes opaque sources, never arbitrary filesystem paths. Record into `filesDir/recordings` with a random ID. App-private files need no storage permission and are removed on uninstall; exporting or sharing can later be a separate explicit action ([app-specific storage](https://developer.android.com/training/data-storage/app-specific)). `start()` creates a new file, `stop()` finalises and publishes it, `cancel()` deletes it, and failures delete partial output. `MediaRecorder.stop()` intentionally throws when no valid samples were captured, and Android instructs applications to remove the malformed output ([`MediaRecorder.stop`](https://developer.android.com/reference/android/media/MediaRecorder#stop())).

Recording is foreground-only in the first version. Android prevents ordinary background applications from accessing the microphone; supporting background recording would require a microphone foreground service and a materially different privacy surface ([MediaRecorder guide](https://developer.android.com/media/platform/mediarecorder)). Navigating away or pausing the activity should stop and finalise an active recording rather than silently continue or discard it. Startup should remove abandoned partial files.

The official Android documentation states that the emulator cannot validate audio capture, so recording quality and microphone routing must be tested on the LP3 itself ([MediaRecorder guide](https://developer.android.com/media/platform/mediarecorder)).

## Size and memory

Audio must remain capability-gated:

| Used feature | Included implementation |
| --- | --- |
| Recorder only | Platform `MediaRecorder`; no Media3 |
| Attached playback | `media3-common` + `media3-exoplayer` and required transitive modules |
| Detached playback | Attached playback + `media3-session` and the service |

The locally cached compressed Media3 1.10.1 AARs are 596 KiB (`common`), 1,660 KiB (`exoplayer`), 896 KiB (`session`), 788 KiB (`extractor`) and 168 KiB (`datasource`). Their 4,108 KiB sum is **not** the APK cost: AARs overlap in dependency purpose, contain metadata and resources, and R8 only retains reachable code. It is evidence that a feature gate matters, not a valid estimate of the final delta. Measure the signed, minified arm64 release APK with and without actual player use before accepting the implementation. Do not quote a debug APK or the downloaded AAR sum as application size.

Runtime policy should be similarly bounded:

- instantiate the player lazily at first use, never at app startup;
- stream remote media rather than copy entire files into Rust or Kotlin memory;
- keep one player and one recorder maximum;
- release attached playback with its screen and detached playback with its service;
- use platform decoders and add no software codec extensions;
- do not add an audio download cache yet;
- start with Media3's defaults, then tune audio-only buffering only if LP3 measurements show a real problem.

## Verification before calling the module complete

Use identical release applications with and without `@ink/audio` and report the APK delta. For memory, use `adb shell dumpsys meminfo` after steady idle, prepared-paused playback, remote playback, detached background playback and active recording. Repeat cold-start and memory samples rather than selecting one run; audio initialisation should also be measured separately from application cold start because the backend is lazy.

Behaviour needs physical LP3 coverage for:

- local asset, recording and HTTPS progressive playback;
- loading, pause/resume, seeking, queue transitions, completion and every public error kind;
- headset disconnection, Bluetooth controls, audio-focus loss and recovery, and incoming calls;
- LightOS now-playing metadata and controls;
- attached screen teardown, detached background playback, reconnecting to a live session, and the idle stop;
- permission grant, denial, blocked denial and revocation;
- very short recording, interrupted recording, low storage, microphone contention and deleting a saved recording.

## Implementation order

1. Add the internal native-controller/event seam and feature detection.
2. Implement attached playback and its typed state using Media3.
3. Add detached playback through `MediaSessionService` and verify LightOS discovery.
4. Add microphone permission and foreground recording with opaque file ownership.
5. Add a compact audio example covering a remote item, a bundled item and recording review.
6. Measure release size, cold start and steady-state memory on the emulator, then validate microphone and routes on a physical LP3.
