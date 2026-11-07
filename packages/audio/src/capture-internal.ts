import { createElement, useEffect, useMemo, useState } from "react";
import type { Snapshot, SnapshotSource } from "ink";
import { NativeError } from "ink/native";
import { useNativeController } from "ink/internal/controller";

const displayController = Symbol("captureController");
export type CaptureDisplay = { readonly [displayController]: number | null };

export function captureReadout(source: { display: CaptureDisplay }, kind: "recorder" | "level" | "pitch") {
  const controller = source.display[displayController];
  return controller === null ? null : createElement("CaptureReadout", { controller, kind });
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

export function useCapture<State extends { status: string; error: Error | null }, Status extends { status: string; error: Error | null }>(
  kind: "recorder" | "level" | "pitch",
  initial: State,
  decode: (value: unknown) => State,
  selectStatus: (value: State) => Status,
  referenceHz?: number,
) {
  const [state, setState] = useState(() => selectStatus(initial));
  const [id, setId] = useState<number | null>(null);
  const controller = useNativeController("audio", "audio capture");
  const channel = useMemo(() => {
    let snapshot: Snapshot<State> = { status: "loading" };
    const listeners = new Set<() => void>();
    const publish = (value: Snapshot<State>) => {
      snapshot = value;
      for (const listener of listeners) listener();
    };
    const observe = () => {
      const active = controller.current;
      if (!active) return;
      void active.call("observeMeasurements", { enabled: listeners.size > 0 }).catch(error => {
        if (controller.current === active) publish({ status: "error", error: error instanceof Error ? error : new Error(String(error)) });
      });
    };
    const source: SnapshotSource<State> = {
      getSnapshot: () => snapshot,
      subscribe(listener) {
        listeners.add(listener);
        if (listeners.size === 1) observe();
        return () => {
          listeners.delete(listener);
          if (!listeners.size) observe();
        };
      },
    };
    return { source, publish, observe };
  }, [controller]);
  useEffect(() => {
    setState(selectStatus(initial));
    setId(null);
    channel.publish({ status: "loading" });
    let statusKey = "";
    const publish = (value: State) => {
      const status = selectStatus(value);
      const key = JSON.stringify([status, status.error?.message]);
      if (key !== statusKey) { statusKey = key; setState(status); }
      channel.publish({ status: "ready", data: value });
    };
    const fail = (error: unknown) => {
      const failure = error instanceof Error ? error : new Error(String(error));
      setState({ ...selectStatus(initial), status: "error", error: failure });
      channel.publish({ status: "error", error: failure });
    };
    return controller.attach({ kind, config: referenceHz === undefined ? {} : { referenceHz } }, value => {
      try { publish(decode(value)); }
      catch (error) { fail(error); }
    }, fail, id => {
      setId(id);
      channel.observe();
    });
  }, [kind, initial, decode, selectStatus, referenceHz, channel, controller]);
  const call = useMemo(() => async (operation: string) => {
    if (!controller.current) throw new NativeError("unavailable", "Audio capture is not attached");
    return controller.current.call(operation);
  }, [controller]);
  const display = useMemo(() => ({ [displayController]: id }), [id]);
  return { state, ready: id !== null, call, display, measurements: channel.source };
}
