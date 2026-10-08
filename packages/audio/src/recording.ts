import { useMemo } from "react";
import { NativeError } from "ink/native";
import type { FileRef } from "@ink/files";
import { captureReadout, fields, useCapture, type CaptureDisplay } from "./capture-internal";

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
const recorderStatus = ({ status, error, recording }: RecorderState) => ({ status, error, recording });
export function useRecorder() {
  const { state, ready, call, display, measurements } = useCapture("recorder", recorderInitial, decodeRecorder, recorderStatus);
  const commands = useMemo(() => ({
    start: async () => { await call("start"); },
    async stop(): Promise<FileRef & { duration: number }> {
      const completed = decodeRecorder(JSON.parse(await call("stop")));
      if (completed.status !== "ready" || !completed.recording) throw new NativeError("protocol", "Recording did not return a saved file");
      return completed.recording;
    },
    cancel: async () => { await call("cancel"); },
    delete: async () => { await call("delete"); },
  }), [call]);
  return { state, ready, display, measurements, ...commands };
}

export function RecordingDuration({ recorder }: { recorder: { display: CaptureDisplay } }) { return captureReadout(recorder, "recorder"); }
