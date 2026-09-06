import { callNative, NativeError } from "ink/native";
import { decodeFileRef } from "./decode";

export interface FileRef {
  id: string;
  src: string;
  name: string;
  mimeType: string;
  size: number;
  width?: number;
  height?: number;
  duration?: number;
}

export const files = {
  async pick(options: { types?: readonly string[]; signal?: AbortSignal } = {}) {
    return decodeFileRef(await callNative("files", "pick", { types: options.types }, { timeoutMs: 600_000, signal: options.signal }));
  },
  open: async (id: string) => decodeFileRef(await callNative("files", "open", { id })),
  remove: async (id: string): Promise<void> => { await callNative("files", "remove", { id }); },
  prepareImage: async (file: FileRef, options: { maxWidth: number; maxHeight: number }): Promise<FileRef> => {
    for (const size of [options.maxWidth, options.maxHeight]) if (!Number.isSafeInteger(size) || size <= 0) throw new RangeError("Image bounds must be positive whole numbers");
    const result = decodeFileRef(await callNative("files", "prepare-image", { id: file.id, ...options }, { timeoutMs: 120_000 }));
    if (!result) throw new NativeError("protocol", "Image preparation returned no file");
    return result;
  },
  save: async (file: FileRef): Promise<void> => { await callNative("files", "save", { id: file.id }, { timeoutMs: 600_000 }); },
  share: async (file: FileRef): Promise<void> => { await callNative("files", "share", { id: file.id }); },
};
