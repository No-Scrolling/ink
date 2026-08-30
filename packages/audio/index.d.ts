// TODO: Import AsyncResource from "ink" once local file packages resolve sibling types correctly.
type ResourceErrorKind =
  | "unavailable"
  | "permission-denied"
  | "permission-blocked"
  | "location-disabled"
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

export interface LevelMeter {
  readonly status: "idle" | "listening" | "active" | "clipping" | "error";
  readonly rms: number;
  readonly peak: number;
  readonly error: string;
  start(): void;
  stop(): void;
}

export interface PitchDetector {
  readonly status: "idle" | "listening" | "active" | "error";
  readonly frequencyHz: number;
  readonly note: string;
  readonly octave: number;
  readonly cents: number;
  readonly confidence: number;
  readonly error: string;
  start(): void;
  stop(): void;
}

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

export interface AudioPlayer {
  readonly status: "idle" | "loading" | "paused" | "playing" | "ended" | "error";
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
  readonly errorKind: "" | "source" | "unsupported" | "output" | "unexpected";
  readonly errorMessage: string;
  readonly errorRetryable: boolean;
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

export interface AudioPlayerOptions {
  readonly usage?: "music" | "speech";
  readonly playback?: "attached" | "detached";
}

export interface AudioRecorder {
  readonly status: "idle" | "recording" | "stopping" | "ready" | "error";
  readonly durationMs: number;
  readonly id: string;
  readonly src: string;
  readonly recordingDurationMs: number;
  readonly errorKind: "" | "permission-denied" | "unavailable" | "output" | "unexpected";
  readonly errorMessage: string;
  readonly errorRetryable: boolean;
  start(): void;
  stop(): void;
  cancel(): void;
  delete(): void;
}

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
