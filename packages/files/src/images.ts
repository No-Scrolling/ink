import { callNative, NativeError } from "ink/native";
import { decodeFileRef } from "./decode";
import type { FileRef } from "./index";

export async function prepareImage(file: FileRef, options: { maxWidth: number; maxHeight: number }): Promise<FileRef> {
  for (const size of [options.maxWidth, options.maxHeight]) if (!Number.isSafeInteger(size) || size < 1 || size > 4096) throw new RangeError("Image bounds must be whole numbers between 1 and 4096");
  const result = decodeFileRef(await callNative("files", "prepare-image", { id: file.id, ...options }, { timeoutMs: 120_000 }));
  if (!result) throw new NativeError("protocol", "Image preparation returned no file");
  return result;
}
