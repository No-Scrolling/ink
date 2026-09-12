import { useEffect, useRef, useState, useCallback } from "react";
import { callNative, NativeError } from "ink/native";
import { attachNativeController } from "ink/native/controller";
import type { Destination } from "ink";

export interface NotificationContent {
  id: string;
  title: string;
  body: string;
  href?: Destination;
  data?: string;
}
async function command(operation: string, payload: unknown): Promise<void> {
  const value: unknown = JSON.parse(await callNative("notifications", operation, payload));
  if (typeof value !== "object" || value === null || !("status" in value)) throw new NativeError("protocol", "Invalid notification result");
  if (value.status === "idle") return;
  if (value.status !== "error" || !("error" in value)) throw new NativeError("protocol", "Invalid notification result");
  throw notificationError(value.error);
}
function notificationError(value: unknown): NativeError {
  if (typeof value !== "object" || value === null
    || !("kind" in value) || typeof value.kind !== "string"
    || !("message" in value) || typeof value.message !== "string") {
    throw new NativeError("protocol", "Invalid notification error");
  }
  return new NativeError(value.kind, value.message, "retryable" in value && value.retryable === true);
}
export const notifications = {
  show(notification: NotificationContent) { return command("schedule", { ...notification, delayMs: 0 }); },
  schedule({ at, ...notification }: NotificationContent & { at: number; exact?: boolean }) {
    if (!Number.isSafeInteger(at) || at < 0) return Promise.reject(new RangeError("Notification time must be a non-negative timestamp in milliseconds"));
    return command("schedule", { ...notification, triggerAtMs: at });
  },
  cancel(id: string) { return command("cancel", { id }); },
};

export interface NotificationTap { id: string; data: string }
type TapState = { status: "loading" } | { status: "empty" } | { status: "ready"; value: NotificationTap } | { status: "error"; error: Error };
function decodeTap(value: unknown): TapState {
  if (typeof value !== "object" || value === null || !("status" in value)) throw new NativeError("protocol", "Invalid notification tap");
  if (value.status === "empty") return { status: "empty" };
  if (value.status === "error" && "error" in value) return { status: "error", error: notificationError(value.error) };
  if (value.status !== "ready" || !("value" in value) || typeof value.value !== "object" || value.value === null
    || !("id" in value.value) || typeof value.value.id !== "string"
    || !("data" in value.value) || typeof value.value.data !== "string") throw new NativeError("protocol", "Invalid notification tap");
  return { status: "ready", value: { id: value.value.id, data: value.value.data } };
}
export function useNotificationTap() {
  const [state, setState] = useState<TapState>({ status: "loading" });
  const controller = useRef<ReturnType<typeof attachNativeController> | null>(null);
  useEffect(() => {
    setState({ status: "loading" });
    const fail = (error: unknown) => setState({ status: "error", error: error instanceof Error ? error : new Error(String(error)) });
    const attachment = attachNativeController("notifications", { kind: "notification-tap" }, value => {
      try { setState(decodeTap(value)); } catch (error) { fail(error); }
    });
    controller.current = attachment;
    void attachment.ready.catch(error => { if (controller.current === attachment) fail(error); });
    return () => {
      controller.current = null;
      void attachment.dispose().catch(error => console.error("Could not release notification taps", error));
    };
  }, []);
  const consume = useCallback(async () => {
    if (!controller.current) throw new NativeError("unavailable", "Notification taps are not attached");
    await controller.current.call("consume");
  }, []);
  return { state, consume };
}
