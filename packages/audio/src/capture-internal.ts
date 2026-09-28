import { createElement, useEffect, useMemo, useRef, useState } from "react";
import { NativeError } from "ink/native";
import { attachNativeController } from "ink/native/controller";

export type CaptureOptions = { updates?: "all" | "status" };
export type CaptureDisplay = { readonly controller: number | null };

export function captureReadout(source: { display: CaptureDisplay }, kind: "recorder" | "level" | "pitch") {
  return source.display.controller === null ? null : createElement("CaptureReadout", { controller: source.display.controller, kind });
}

export function fields(value: unknown) {
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

export function useCapture<State extends { status: string; error: Error | null }>(
  kind: "recorder" | "level" | "pitch",
  initial: State,
  decode: (value: unknown) => State,
  referenceHz?: number,
  updates: "all" | "status" = "all",
) {
  const [state, setState] = useState(initial);
  const [id, setId] = useState<number | null>(null);
  const controller = useRef<ReturnType<typeof attachNativeController> | null>(null);
  useEffect(() => {
    setState(initial);
    setId(null);
    const fail = (error: unknown) => setState({ ...initial, status: "error", error: error instanceof Error ? error : new Error(String(error)) });
    const attachment = attachNativeController("audio", { kind, config: { ...(referenceHz === undefined ? {} : { referenceHz }), updates } }, value => {
      try { setState(decode(value)); }
      catch (error) { fail(error); }
    });
    controller.current = attachment;
    void attachment.ready.then(() => {
      if (controller.current === attachment) setId(attachment.id);
    }, error => {
      if (controller.current === attachment) fail(error);
    });
    return () => {
      controller.current = null;
      void attachment.dispose().catch(error => console.error("Could not release audio capture", error));
    };
  }, [kind, initial, decode, referenceHz, updates]);
  const call = useMemo(() => async (operation: string) => {
    if (!controller.current) throw new NativeError("unavailable", "Audio capture is not attached");
    await controller.current.call(operation);
  }, []);
  const display = useMemo(() => ({ controller: id }), [id]);
  return { state, ready: id !== null, call, display };
}
