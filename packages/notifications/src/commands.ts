import { callNative, NativeError } from "ink/native";
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
export function notificationError(value: unknown): NativeError {
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

