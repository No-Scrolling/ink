import { callNative } from "ink/native";

export const secureStore = {
  async get(key: string): Promise<string | null> {
    const value: unknown = JSON.parse(await callNative("secure-store", "get", { key }));
    if (value !== null && typeof value !== "string") throw new Error("Invalid secure store result");
    return value;
  },
  async set(key: string, value: string): Promise<void> {
    await callNative("secure-store", "set", { key, value });
  },
  async remove(key: string): Promise<void> {
    await callNative("secure-store", "remove", { key });
  },
};
