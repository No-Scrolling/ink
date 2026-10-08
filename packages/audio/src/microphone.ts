import { createElement, useMemo } from "react";
import { NativeError } from "ink/native";
import { captureReadout, fields, useCapture, type CaptureDisplay } from "./capture-internal";

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
const levelStatus = ({ status, error }: LevelState) => ({ status, error });
export function useLevelMeter() {
  const { state, ready, call, display, measurements } = useCapture("level", levelInitial, decodeLevel, levelStatus);
  const commands = useMemo(() => ({ start: async () => { await call("start"); }, stop: async () => { await call("stop"); } }), [call]);
  return { state, ready, display, measurements, ...commands };
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
const pitchStatus = ({ status, error }: PitchState) => ({ status, error });
export function usePitchDetector({ referenceHz = 440 }: { referenceHz?: number } = {}) {
  if (!Number.isFinite(referenceHz) || referenceHz <= 0) throw new RangeError("Pitch reference must be a positive frequency");
  const { state, ready, call, display, measurements } = useCapture("pitch", pitchInitial, decodePitch, pitchStatus, referenceHz);
  const commands = useMemo(() => ({ start: async () => { await call("start"); }, stop: async () => { await call("stop"); } }), [call]);
  return { state, ready, display, measurements, ...commands };
}

export function LevelReadout({ meter }: { meter: { display: CaptureDisplay } }) { return captureReadout(meter, "level"); }
export function PitchReadout({ detector }: { detector: { display: CaptureDisplay } }) { return captureReadout(detector, "pitch"); }

export function PitchIndicator({ cents }: { cents: number | null }) {
  return createElement("PitchIndicator", { cents });
}
