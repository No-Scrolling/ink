import { callNative, NativeError } from "ink/native";

export interface PushMessage {
  id: string;
  groupKey: string;
  title: string;
  body: string;
  route: string;
  receivedAt: number;
}
export function fields(value: unknown) {
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
export function decodeMessage(value: unknown): PushMessage {
  const item = fields(value);
  return { id: item.text("id"), groupKey: item.text("groupKey"), title: item.text("title"), body: item.text("body"), route: item.text("route"), receivedAt: item.time("receivedAtMs") };
}
export interface PushDelivery { messages: readonly PushMessage[]; cancelled: readonly string[] }
export function decodePushDelivery(value: unknown): PushDelivery {
  if (typeof value !== "object" || value === null || !("messages" in value) || !Array.isArray(value.messages)
    || !("cancelled" in value) || !Array.isArray(value.cancelled)) throw new TypeError("Invalid push delivery");
  return {
    messages: value.messages.map(decodeMessage),
    cancelled: value.cancelled.map((key: unknown) => {
      if (typeof key !== "string") throw new TypeError("Invalid cancelled push group");
      return key;
    }),
  };
}
export async function setPushTask(task: { id: string } | null): Promise<void> {
  await callNative("background", "set-push-task", { task: task?.id ?? null });
}
