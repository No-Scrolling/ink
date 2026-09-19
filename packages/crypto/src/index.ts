import { callNative } from "ink/native";

export function createCipher(key: string) {
  const transform = async (operation: "encrypt" | "decrypt", text: string): Promise<string> =>
    JSON.parse(await callNative("crypto", operation, { key, text }));
  return {
    encrypt: (text: string) => transform("encrypt", text),
    decrypt: (text: string) => transform("decrypt", text),
  };
}
