import { callNative } from "ink/native";
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
  save: async (file: FileRef): Promise<void> => { await callNative("files", "save", { id: file.id }, { timeoutMs: 600_000 }); },
  share: async (file: FileRef): Promise<void> => { await callNative("files", "share", { id: file.id }); },
};
