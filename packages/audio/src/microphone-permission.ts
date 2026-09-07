import { callNative, NativeError } from "ink/native";

export type MicrophonePermission = "granted" | "denied" | "blocked";
async function permission(operation: string): Promise<MicrophonePermission> {
  const value = await callNative("permissions", operation, { permission: "microphone" }, { timeoutMs: 120_000 });
  if (value !== "granted" && value !== "denied" && value !== "blocked") {
    throw new NativeError("protocol", "Invalid microphone permission result");
  }
  return value;
}
export const microphone = {
  getPermission: () => permission("status"),
  requestPermission: () => permission("request"),
};
