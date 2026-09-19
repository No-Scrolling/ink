import { callNative, NativeError } from "ink/native";

export type Permission = "camera" | "microphone" | "location-approximate" | "location-precise" | "notifications" | "photos" | "videos" | "photos-and-videos" | "audio-files";
export type PermissionStatus = "granted" | "denied" | "blocked";
export interface HostPreferences { hapticsEnabled: boolean }
export interface KeyboardOptions {
  emojis: string | null;
  displayVoice: boolean;
  enableKeyAnimation: boolean;
  swipeEnabled: boolean | null;
}
async function permission(operation: string, name: Permission, signal?: AbortSignal): Promise<PermissionStatus> {
  const value = await callNative("permissions", operation, { permission: name }, { timeoutMs: 120_000, signal });
  if (value !== "granted" && value !== "denied" && value !== "blocked") throw new NativeError("protocol", "Invalid permission result");
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
  getPermission(name: Permission, { signal }: { signal?: AbortSignal } = {}) { return permission("status", name, signal); },
  requestPermission(name: Permission, { signal }: { signal?: AbortSignal } = {}) { return permission("request", name, signal); },
  async canScheduleExact(): Promise<boolean> {
    const value = await callNative("notifications", "exact-status", {});
    if (value !== "granted" && value !== "denied") throw new NativeError("protocol", "Invalid exact alarm permission result");
    return value === "granted";
  },
  async requestExactPermission(): Promise<void> {
    await callNative("notifications", "request-exact-permission", {});
  },
  async openDialler({ phoneNumber }: { phoneNumber: string }): Promise<void> {
    if (!phoneNumber.trim() || phoneNumber.length > 64 || /[\u0000-\u001f\u007f-\u009f]/.test(phoneNumber)) {
      throw new TypeError("Phone number must contain 1–64 characters without control characters");
    }
    await callNative("light-sdk", "open-dialler", { phoneNumber });
  },
};
