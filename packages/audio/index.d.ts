import type { InkError, PermissionResource } from "ink";

export type MicrophonePermission = PermissionResource;

export type AudioCaptureErrorKind =
  | "permission-denied"
  | "unavailable"
  | "input"
  | "unexpected";

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
    | {
        readonly status: "error";
        readonly error: InkError<AudioCaptureErrorKind>;
      }
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
    | {
        readonly status: "error";
        readonly error: InkError<AudioCaptureErrorKind>;
      }
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

export type AudioPlayerErrorKind =
  | "source"
  | "unsupported"
  | "output"
  | "unexpected";

export type AudioPlayer = AudioPlayerActions &
  AudioPlayerValue &
  (
    | { readonly status: "idle" | "loading" | "paused" | "playing" | "ended" }
    | {
        readonly status: "error";
        readonly error: InkError<AudioPlayerErrorKind>;
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

export type AudioRecorderErrorKind =
  | "permission-denied"
  | "unavailable"
  | "output"
  | "unexpected";

export type AudioRecorder = AudioRecorderActions &
  AudioRecorderValue &
  (
    | { readonly status: "idle" | "recording" | "stopping" | "ready" }
    | {
        readonly status: "error";
        readonly error: InkError<AudioRecorderErrorKind>;
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
