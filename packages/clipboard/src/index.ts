import { callNative, NativeError } from "ink/native";

export const clipboard = {
  async writeText(text: string): Promise<void> {
    await callNative("clipboard", "write-text", { text });
  },
  async readText(): Promise<string | null> {
    const value: unknown = JSON.parse(await callNative("clipboard", "read-text", {}));
    if (value !== null && typeof value !== "string") {
      throw new NativeError("protocol", "Invalid clipboard text result");
    }
    return value;
  },
};
