import { useEffect, useMemo, useRef, useState } from "react";
import { callNative, NativeError } from "ink/native";
import { attachNativeController } from "ink/native/controller";

export interface PushMessage {
  id: string;
  groupKey: string;
  title: string;
  body: string;
  route: string;
  receivedAt: number;
}
export interface PushState {
  status: "idle" | "registering" | "synchronising" | "ready" | "error";
  endpoint: string;
  registeredAt: number;
  openedKey: string;
  messages: readonly PushMessage[];
  error: Error | null;
}
const initial: PushState = { status: "idle", endpoint: "", registeredAt: 0, openedKey: "", messages: [], error: null };
function fields(value: unknown) {
  if (typeof value !== "object" || value === null) throw new NativeError("protocol", "Invalid push state");
  return {
    text(name: string) {
      const field: unknown = Reflect.get(value, name);
      if (typeof field !== "string") throw new NativeError("protocol", `Invalid push ${name}`);
      return field;
    },
    time(name: string) {
      const field: unknown = Reflect.get(value, name);
      if (typeof field !== "number" || !Number.isSafeInteger(field) || field < 0) throw new NativeError("protocol", `Invalid push ${name}`);
      return field;
    },
  };
}
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
    messages: value.messages.map((message: unknown) => {
      const item = fields(message);
      return { id: item.text("id"), groupKey: item.text("groupKey"), title: item.text("title"), body: item.text("body"), route: item.text("route"), receivedAt: item.time("receivedAtMs") };
    }) };
}

export function usePush() {
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
  const commands = useMemo(() => {
    const call = async (operation: string, payload: unknown = {}) => {
      if (!controller.current) throw new NativeError("unavailable", "Push inbox is not attached");
      await controller.current.call(operation, payload);
    };
    return {
      register: (subscriptionBaseUrl: string, bearerToken = "") => call("register", { subscriptionBaseUrl, bearerToken }),
      retry: () => call("retry"), unregister: () => call("unregister"),
      dismiss: (groupKey: string) => call("dismiss", { groupKey }), clear: () => call("clear"),
    };
  }, []);
  return { state, ready, ...commands };
}

export interface PushDelivery { messages: readonly PushMessage[]; cancelled: readonly string[] }
export function decodePushDelivery(value: unknown): PushDelivery {
  if (typeof value !== "object" || value === null || !("messages" in value) || !Array.isArray(value.messages)
    || !("cancelled" in value) || !Array.isArray(value.cancelled)) throw new TypeError("Invalid push delivery");
  return {
    messages: value.messages.map((message: unknown) => {
      const item = fields(message);
      return { id: item.text("id"), groupKey: item.text("groupKey"), title: item.text("title"), body: item.text("body"), route: item.text("route"), receivedAt: item.time("receivedAtMs") };
    }),
    cancelled: value.cancelled.map((key: unknown) => {
      if (typeof key !== "string") throw new TypeError("Invalid cancelled push group");
      return key;
    }),
  };
}
export async function setPushTask(task: { id: string } | null): Promise<void> {
  await callNative("background", "set-push-task", { task: task?.id ?? null });
}
