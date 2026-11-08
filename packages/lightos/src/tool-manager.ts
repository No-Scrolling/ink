import { useEffect, useState } from "ink/internal/react";
import { callNative, callNativeBytes, NativeError } from "ink/native";

export interface SharedFile {
  name: string;
  path: string;
  uri: string;
  isFile: boolean;
  size: number;
  lastModified: number;
}

function decodeFiles(source: string): SharedFile[] {
  const files: unknown = JSON.parse(source);
  if (!Array.isArray(files)) throw new NativeError("protocol", "Invalid shared file list");
  return files.map((file: unknown) => {
    if (typeof file !== "object" || file === null
      || !["name", "path", "uri"].every(key => typeof Reflect.get(file, key) === "string")
      || typeof Reflect.get(file, "isFile") !== "boolean"
      || !["size", "lastModified"].every(key => {
        const value: unknown = Reflect.get(file, key);
        return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
      })) throw new NativeError("protocol", "Invalid shared file");
    return file as SharedFile;
  });
}

export async function setSharedFilesTask(task: { id: string } | null): Promise<void> {
  await callNative("background", "set-shared-files-task", { task: task?.id ?? null });
}

export const sharedFiles = {
  async list(path = ""): Promise<SharedFile[]> {
    return decodeFiles(await callNative("tool-manager", "list", { path }));
  },
  async exists(path: string): Promise<boolean> {
    return await callNative("tool-manager", "exists", { path }) === "true";
  },
  async readBytes(path: string, options: { offset?: number; length?: number; signal?: AbortSignal } = {}): Promise<Uint8Array> {
    if (options.offset !== undefined && (!Number.isSafeInteger(options.offset) || options.offset < 0)) throw new RangeError("Invalid shared file offset");
    if (options.length !== undefined && (!Number.isSafeInteger(options.length) || options.length < 0 || options.length > 524_288)) throw new RangeError("Shared reads accept up to 512 KiB");
    return (await callNativeBytes("tool-manager", "read", { path, offset: options.offset, length: options.length }, { signal: options.signal })).bytes;
  },
  async writeBytes(path: string, bytes: Uint8Array): Promise<void> {
    await callNativeBytes("tool-manager", "write", { path }, { bytes });
  },
  async copyFrom(path: string, source: string, options: { signal?: AbortSignal } = {}): Promise<void> {
    await callNative("tool-manager", "copy-from", { path, source }, { signal: options.signal, timeoutMs: 600_000 });
  },
  async readText(path: string): Promise<string> {
    return await callNative("tool-manager", "read-text", { path });
  },
  async writeText(path: string, text: string): Promise<void> {
    await callNative("tool-manager", "write-text", { path, text });
  },
  async remove(path: string): Promise<boolean> {
    return await callNative("tool-manager", "remove", { path }) === "true";
  },
  async mkdir(path: string): Promise<void> {
    await callNative("tool-manager", "mkdir", { path });
  },
};

export function useSharedFiles(path = "") {
  const [state, setState] = useState<{ files: SharedFile[]; loading: boolean; error: Error | null }>({
    files: [], loading: true, error: null,
  });
  useEffect(() => {
    const controller = new AbortController();
    setState({ files: [], loading: true, error: null });
    void (async () => {
      let revision = -1;
      while (!controller.signal.aborted) {
        const next = Number(await callNative("tool-manager", "observe", { revision }, { signal: controller.signal }));
        if (!Number.isSafeInteger(next) || next < 0) throw new NativeError("protocol", "Invalid shared file state");
        if (next !== revision) {
          const files = await sharedFiles.list(path);
          if (!controller.signal.aborted) setState({ files, loading: false, error: null });
        }
        revision = next;
      }
    })().catch(error => {
      if (!controller.signal.aborted) setState(current => ({ ...current, loading: false,
        error: error instanceof Error ? error : new Error(String(error)) }));
    });
    return () => controller.abort();
  }, [path]);
  return state;
}
