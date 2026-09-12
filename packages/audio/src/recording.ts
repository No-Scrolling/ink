import { useMemo } from "react";
import { NativeError } from "ink/native";
import type { FileRef } from "@ink/files";
import { fields, useCapture } from "./capture-internal";

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
