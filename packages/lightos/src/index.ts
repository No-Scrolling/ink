import { callNative, NativeError } from "ink/native";

export type HostPermission = "camera" | "microphone" | "location-approximate" | "location-precise";
export type PermissionStatus = "granted" | "denied" | "blocked";
export interface HostPreferences { hapticsEnabled: boolean }
export interface KeyboardOptions {
  emojis: string | null;
  displayVoice: boolean;
  enableKeyAnimation: boolean;
  swipeEnabled: boolean | null;
}
async function permission(operation: string, name: HostPermission): Promise<PermissionStatus> {
  const value = await callNative("permissions", operation, { permission: name, hostRequired: true }, { timeoutMs: 120_000 });
  if (value !== "granted" && value !== "denied" && value !== "blocked") throw new NativeError("protocol", "Invalid host permission result");
  return value;
}

export const lightos = {
  async getPreferences(): Promise<HostPreferences> {
    const value: unknown = JSON.parse(await callNative("light-sdk", "preferences", ""));
    if (typeof value !== "object" || value === null || !("hapticsEnabled" in value)
      || typeof value.hapticsEnabled !== "boolean") throw new NativeError("protocol", "Invalid host preferences");
    return { hapticsEnabled: value.hapticsEnabled };
  },
  async getKeyboardOptions(): Promise<KeyboardOptions> {
    const value: unknown = JSON.parse(await callNative("light-sdk", "keyboard-options", ""));
    if (typeof value !== "object" || value === null
      || !("displayVoice" in value) || typeof value.displayVoice !== "boolean"
      || !("enableKeyAnimation" in value) || typeof value.enableKeyAnimation !== "boolean"
      || ("emojisAsString" in value && value.emojisAsString !== null && typeof value.emojisAsString !== "string")
      || ("swipeEnabled" in value && value.swipeEnabled !== null && typeof value.swipeEnabled !== "boolean")) {
      throw new NativeError("protocol", "Invalid host keyboard options");
    }
    return {
      emojis: "emojisAsString" in value && typeof value.emojisAsString === "string" ? value.emojisAsString : null,
      displayVoice: value.displayVoice,
      enableKeyAnimation: value.enableKeyAnimation,
      swipeEnabled: "swipeEnabled" in value && typeof value.swipeEnabled === "boolean" ? value.swipeEnabled : null,
    };
  },
  async getVersion(): Promise<string> {
    const version = await callNative("light-sdk", "version", "");
    if (!version.trim()) throw new NativeError("protocol", "Host returned an empty version");
    return version;
  },
  getPermission(name: HostPermission) { return permission("status", name); },
  requestPermission(name: HostPermission) { return permission("request", name); },
  async openDialler({ phoneNumber }: { phoneNumber: string }): Promise<void> {
    if (!phoneNumber.trim() || phoneNumber.length > 64 || /[\u0000-\u001f\u007f-\u009f]/.test(phoneNumber)) {
      throw new TypeError("Phone number must contain 1–64 characters without control characters");
    }
    await callNative("light-sdk", "open-dialler", { phoneNumber });
  },
};
