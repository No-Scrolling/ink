import { useEffect, useMemo, useRef, useState } from "react";
import { callNative, NativeError } from "ink/native";
import { attachNativeController } from "ink/native/controller";
import type { FileRef } from "@ink/files";

export type MicrophonePermission = "granted" | "denied" | "blocked";
async function permission(operation: string): Promise<MicrophonePermission> {
  const value = await callNative("permissions", operation, { permission: "microphone" }, { timeoutMs: 120_000 });
  if (value !== "granted" && value !== "denied" && value !== "blocked") {
    throw new NativeError("protocol", "Invalid microphone permission result");
  }
  return value;
}
export const microphone = {
  getPermission: () => permission("status"),
  requestPermission: () => permission("request"),
};

function fields(value: unknown) {
  if (typeof value !== "object" || value === null) throw new NativeError("protocol", "Invalid audio capture state");
  return {
    text(name: string) {
      const result: unknown = Reflect.get(value, name);
      if (typeof result !== "string") throw new NativeError("protocol", `Invalid audio capture ${name}`);
      return result;
    },
    number(name: string, minimum = 0) {
      const result: unknown = Reflect.get(value, name);
      if (typeof result !== "number" || !Number.isFinite(result) || result < minimum) {
        throw new NativeError("protocol", `Invalid audio capture ${name}`);
      }
      return result;
    },
    error(): Error | null {
      if (Reflect.get(value, "status") !== "error") return null;
      const detail: unknown = Reflect.get(value, "error");
      if (typeof detail !== "object" || detail === null
        || !("kind" in detail) || typeof detail.kind !== "string"
        || !("message" in detail) || typeof detail.message !== "string") {
        throw new NativeError("protocol", "Invalid audio capture error");
      }
      return new NativeError(detail.kind, detail.message, "retryable" in detail && detail.retryable === true);
    },
  };
}

function useCapture<State extends { status: string; error: Error | null }>(
  kind: "recorder" | "level" | "pitch",
  initial: State,
  decode: (value: unknown) => State,
  referenceHz?: number,
) {
  const [state, setState] = useState(initial);
  const [ready, setReady] = useState(false);
  const controller = useRef<ReturnType<typeof attachNativeController> | null>(null);
  useEffect(() => {
    setState(initial);
    setReady(false);
    const fail = (error: unknown) => setState({ ...initial, status: "error", error: error instanceof Error ? error : new Error(String(error)) });
    const attachment = attachNativeController("audio", { kind, config: referenceHz === undefined ? {} : { referenceHz } }, value => {
      try { setState(decode(value)); }
      catch (error) { fail(error); }
    });
    controller.current = attachment;
    void attachment.ready.then(() => {
      if (controller.current === attachment) setReady(true);
    }, error => {
      if (controller.current === attachment) fail(error);
    });
    return () => {
      controller.current = null;
      void attachment.dispose().catch(error => console.error("Could not release audio capture", error));
    };
  }, [kind, initial, decode, referenceHz]);
  const call = useMemo(() => async (operation: string) => {
    if (!controller.current) throw new NativeError("unavailable", "Audio capture is not attached");
    await controller.current.call(operation);
  }, []);
  return { state, ready, call };
}

export interface RecorderState {
  status: "idle" | "recording" | "stopping" | "ready" | "error";
  duration: number;
  recording: (FileRef & { duration: number }) | null;
  error: Error | null;
}
const recorderInitial: RecorderState = { status: "idle", duration: 0, recording: null, error: null };
function decodeRecorder(value: unknown): RecorderState {
  const field = fields(value);
  const status = field.text("status");
  if (status !== "idle" && status !== "recording" && status !== "stopping" && status !== "ready" && status !== "error") {
    throw new NativeError("protocol", "Invalid recorder status");
  }
  const id = field.text("id");
  return { status, duration: field.number("durationMs"), error: field.error(),
    recording: id ? { id, src: `ink-file://${id}`, name: field.text("name"), mimeType: field.text("mimeType"), size: field.number("size"), duration: field.number("recordingDurationMs") } : null };
}
export function useRecorder() {
  const { state, ready, call } = useCapture("recorder", recorderInitial, decodeRecorder);
  const commands = useMemo(() => ({
    start: () => call("start"), stop: () => call("stop"),
    cancel: () => call("cancel"), delete: () => call("delete"),
  }), [call]);
  return { state, ready, ...commands };
}

export interface LevelState {
  status: "idle" | "listening" | "active" | "clipping" | "error";
  rms: number;
  peak: number;
  error: Error | null;
}
const levelInitial: LevelState = { status: "idle", rms: 0, peak: 0, error: null };
function decodeLevel(value: unknown): LevelState {
  const field = fields(value);
  const status = field.text("status");
  if (status !== "idle" && status !== "listening" && status !== "active" && status !== "clipping" && status !== "error") {
    throw new NativeError("protocol", "Invalid level meter status");
  }
  return { status, rms: field.number("rms"), peak: field.number("peak"), error: field.error() };
}
export function useLevelMeter() {
  const { state, ready, call } = useCapture("level", levelInitial, decodeLevel);
  const commands = useMemo(() => ({ start: () => call("start"), stop: () => call("stop") }), [call]);
  return { state, ready, ...commands };
}

export interface PitchState {
  status: "idle" | "listening" | "active" | "error";
  frequency: number;
  note: string;
  octave: number;
  cents: number;
  confidence: number;
  error: Error | null;
}
const pitchInitial: PitchState = { status: "idle", frequency: 0, note: "", octave: 0, cents: 0, confidence: 0, error: null };
function decodePitch(value: unknown): PitchState {
  const field = fields(value);
  const status = field.text("status");
  if (status !== "idle" && status !== "listening" && status !== "active" && status !== "error") {
    throw new NativeError("protocol", "Invalid pitch detector status");
  }
  return { status, frequency: field.number("frequencyHz"), note: field.text("note"),
    octave: field.number("octave", -Infinity), cents: field.number("cents", -Infinity),
    confidence: field.number("confidence"), error: field.error() };
}
export function usePitchDetector({ referenceHz = 440 }: { referenceHz?: number } = {}) {
  if (!Number.isFinite(referenceHz) || referenceHz <= 0) throw new RangeError("Pitch reference must be a positive frequency");
  const { state, ready, call } = useCapture("pitch", pitchInitial, decodePitch, referenceHz);
  const commands = useMemo(() => ({ start: () => call("start"), stop: () => call("stop") }), [call]);
  return { state, ready, ...commands };
}
