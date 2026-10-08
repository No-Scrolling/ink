import { NativeError } from "ink/native";
import { useSession, type CapturedPhoto } from "./shared";
export { CameraPreview, type CapturedPhoto } from "./shared";

function decodePhoto(value: unknown): CapturedPhoto {
  if (typeof value !== "object" || value === null
    || !("width" in value) || typeof value.width !== "number" || !Number.isSafeInteger(value.width) || value.width <= 0
    || !("height" in value) || typeof value.height !== "number" || !Number.isSafeInteger(value.height) || value.height <= 0
    || !("mimeType" in value) || value.mimeType !== "image/jpeg"
    || !("capturedAtMs" in value) || typeof value.capturedAtMs !== "number" || !Number.isSafeInteger(value.capturedAtMs)) throw new NativeError("protocol", "Invalid captured photo");
  if (!("file" in value) || typeof value.file !== "object" || value.file === null
    || !("name" in value.file) || typeof value.file.name !== "string"
    || !("size" in value.file) || typeof value.file.size !== "number" || !Number.isSafeInteger(value.file.size) || value.file.size < 0
    || !("mimeType" in value.file) || value.file.mimeType !== "image/jpeg") throw new NativeError("protocol", "Invalid captured file");
  if (!("id" in value.file) || typeof value.file.id !== "string" || !("src" in value.file) || typeof value.file.src !== "string") throw new NativeError("protocol", "Invalid managed photo");
  const file: CapturedPhoto["file"] = { id: value.file.id, src: value.file.src, width: value.width, height: value.height, name: value.file.name, size: value.file.size, mimeType: value.file.mimeType };
  return { file, capturedAt: value.capturedAtMs };
}

export function useCamera({ facing = "back" }: { facing?: "back" | "front" } = {}) {
  return useSession("photo", decodePhoto, JSON.stringify({ facing }));
}
