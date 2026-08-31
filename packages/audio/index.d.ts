// TODO: Import AsyncResource from "ink" once local file packages resolve sibling types correctly.
type ResourceErrorKind =
  | "unavailable"
  | "permission-denied"
  | "permission-blocked"
  | "location-disabled"
  | "nfc-disabled"
  | "timeout"
  | "protocol"
  | "unexpected";

interface ResourceError {
  readonly kind: ResourceErrorKind;
  readonly message: string;
  readonly retryable: boolean;
}

type AsyncResource<T> = {
  reload(): void;
} &
  (
    | { readonly status: "loading" }
    | { readonly status: "ready"; readonly value: T }
    | { readonly status: "error"; readonly error: ResourceError }
  );

export type MicrophonePermission = AsyncResource<
  "granted" | "denied" | "blocked" | "unknown"
> & {
  request(): void;
};

interface LevelMeterValue {
  readonly rms: number;
  readonly peak: number;
}

interface LevelMeterActions {
  start(): void;
  stop(): void;
}

export type LevelMeter = LevelMeterActions &
  LevelMeterValue &
  (
    | { readonly status: "idle" | "listening" | "active" | "clipping" }
    | { readonly status: "error"; readonly error: string }
  );

interface PitchDetectorValue {
  readonly frequencyHz: number;
  readonly note: string;
  readonly octave: number;
  readonly cents: number;
  readonly confidence: number;
}

interface PitchDetectorActions {
  start(): void;
  stop(): void;
}

export type PitchDetector = PitchDetectorActions &
  PitchDetectorValue &
  (
    | { readonly status: "idle" | "listening" | "active" }
    | { readonly status: "error"; readonly error: string }
  );

export interface PitchDetectorOptions {
  readonly referenceHz?: number;
}

export interface AudioItem {
  readonly id?: string;
  readonly src: string;
  readonly title: string;
  readonly artist?: string;
  readonly album?: string;
  readonly artwork?: string;
}

interface AudioPlayerValue {
  readonly id: string;
  readonly src: string;
  readonly title: string;
  readonly artist: string;
  readonly album: string;
  readonly artwork: string;
  readonly index: number;
  readonly positionMs: number;
  readonly durationMs: number;
  readonly bufferedMs: number;
  readonly speed: number;
}

interface AudioPlayerActions {
  /** Starts the supplied item, or resumes the current queue when omitted. */
  play(item?: AudioItem): void;
  /** Starts the most recently saved Ink recording. */
  playRecording(): void;
  /** Replaces the queue without starting playback. */
  setQueue(items: ReadonlyArray<AudioItem>, startIndex?: number): void;
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

export type AudioPlayer = AudioPlayerActions &
  AudioPlayerValue &
  (
    | { readonly status: "idle" | "loading" | "paused" | "playing" | "ended" }
    | {
        readonly status: "error";
        readonly errorKind: "source" | "unsupported" | "output" | "unexpected";
        readonly errorMessage: string;
        readonly errorRetryable: boolean;
      }
  );

export interface AudioPlayerOptions {
  readonly usage?: "music" | "speech";
  readonly playback?: "attached" | "detached";
}

interface AudioRecorderValue {
  readonly durationMs: number;
  readonly id: string;
  readonly src: string;
  readonly recordingDurationMs: number;
}

interface AudioRecorderActions {
  start(): void;
  stop(): void;
  cancel(): void;
  delete(): void;
}

export type AudioRecorder = AudioRecorderActions &
  AudioRecorderValue &
  (
    | { readonly status: "idle" | "recording" | "stopping" | "ready" }
    | {
        readonly status: "error";
        readonly errorKind:
          | "permission-denied"
          | "unavailable"
          | "output"
          | "unexpected";
        readonly errorMessage: string;
        readonly errorRetryable: boolean;
      }
  );

/** Reads and requests the microphone permission used by every capture controller. */
export declare function microphonePermission(): MicrophonePermission;
/** Measures normalised RMS and peak levels from a shared microphone capture. */
export declare function levelMeter(): LevelMeter;
/** Detects a monophonic pitch from a shared microphone capture. */
export declare function pitchDetector(options?: PitchDetectorOptions): PitchDetector;
/** Creates one screen-scoped attached or detached player. */
export declare function audioPlayer(options?: AudioPlayerOptions): AudioPlayer;
/** Creates one screen-scoped AAC/M4A recorder. */
export declare function audioRecorder(): AudioRecorder;
