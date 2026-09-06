import { callNative, NativeError } from "ink/native";

export type RingtoneKind = "ringtone" | "notification" | "alarm";

export async function installRingtone(
  source: string,
  kind: RingtoneKind = "ringtone",
  { signal }: { signal?: AbortSignal } = {},
): Promise<void> {
  const value: unknown = JSON.parse(await callNative("light-sdk", "install-ringtone", { source, kind }, { signal }));
  if (typeof value !== "object" || value === null || !("status" in value)) throw new NativeError("protocol", "Invalid ringtone result");
  if (value.status === "installed") return;
  if (value.status !== "error" || !("error" in value) || typeof value.error !== "object" || value.error === null
    || !("kind" in value.error) || typeof value.error.kind !== "string"
    || !("message" in value.error) || typeof value.error.message !== "string") throw new NativeError("protocol", "Invalid ringtone error");
  throw new NativeError(value.error.kind, value.error.message, "retryable" in value.error && value.error.retryable === true);
}
