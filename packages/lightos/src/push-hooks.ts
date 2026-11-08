import { fields, decodeMessage, type PushMessage } from "./push-delivery";
import { useEffect, useMemo, useRef, useState } from "ink/internal/react";
import { NativeError } from "ink/native";
import { attachNativeController } from "ink/native/controller";

export interface PushState {
  status: "idle" | "registering" | "synchronising" | "ready" | "error";
  endpoint: string;
  registeredAt: number;
  openedKey: string;
  messages: readonly PushMessage[];
  error: Error | null;
}
export interface PushRegistration {
  url: string;
  bearerToken?: string;
  encryptionKey?: string;
  /** Fetch the server inbox after a wake-up, with or without encryption. Unusable bodies also trigger a fetch. */
  fetchOnWake?: boolean;
}
export interface PushOptions {
  foregroundGroup?: string | null;
}
const initial: PushState = { status: "idle", endpoint: "", registeredAt: 0, openedKey: "", messages: [], error: null };
function decode(value: unknown): PushState {
  const field = fields(value);
  const status = field.text("status");
  if (status !== "idle" && status !== "registering" && status !== "synchronising" && status !== "ready" && status !== "error") throw new NativeError("protocol", "Invalid push status");
  if (typeof value !== "object" || value === null || !("messages" in value) || !Array.isArray(value.messages)) throw new NativeError("protocol", "Invalid push messages");
  let error: Error | null = null;
  if (status === "error") {
    if (!("error" in value) || typeof value.error !== "object" || value.error === null) throw new NativeError("protocol", "Invalid push error");
    const detail = fields(value.error);
    error = new NativeError(detail.text("kind"), detail.text("message"), "retryable" in value.error && value.error.retryable === true);
  }
  return { status, endpoint: field.text("endpoint"), registeredAt: field.time("registeredAtMs"), openedKey: field.text("openedKey"), error,
    messages: value.messages.map(decodeMessage) };
}

export function usePush({ foregroundGroup = null }: PushOptions = {}) {
  const [state, setState] = useState(initial);
  const [ready, setReady] = useState(false);
  const controller = useRef<ReturnType<typeof attachNativeController> | null>(null);
  useEffect(() => {
    setState(initial);
    setReady(false);
    const fail = (error: unknown) => setState({ ...initial, status: "error", error: error instanceof Error ? error : new Error(String(error)) });
    const attachment = attachNativeController("notifications", { kind: "light-push" }, value => {
      try { setState(decode(value)); } catch (error) { fail(error); }
    });
    controller.current = attachment;
    void attachment.ready.then(() => {
      if (controller.current === attachment) setReady(true);
    }, error => { if (controller.current === attachment) fail(error); });
    return () => {
      controller.current = null;
      void attachment.dispose().catch(error => console.error("Could not release push inbox", error));
    };
  }, []);
  useEffect(() => {
    const attachment = controller.current;
    if (!attachment) return;
    let active = true;
    void attachment.call("foreground", { groupKey: foregroundGroup }).catch(error => {
      if (active) setState(current => ({ ...current, status: "error", error: error instanceof Error ? error : new Error(String(error)) }));
    });
    return () => { active = false; };
  }, [foregroundGroup]);
  const commands = useMemo(() => {
    const call = async (operation: string, payload: unknown = {}) => {
      if (!controller.current) throw new NativeError("unavailable", "Push inbox is not attached");
      await controller.current.call(operation, payload);
    };
    return {
      register: (options: PushRegistration) => call("register", options),
      retry: () => call("retry"), unregister: () => call("unregister"),
      dismiss: (groupKey: string) => call("dismiss", { groupKey }), clear: () => call("clear"),
      notifyForeground: (groupKey: string) => call("foreground-message", { groupKey }),
    };
  }, []);
  return { state, ready, ...commands };
}

