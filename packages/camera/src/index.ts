import { NativeError } from "ink/native";
import { useSession, type CapturedFile, type CapturedPhoto } from "./shared";
export { camera, CameraPreview, type CapturedFile, type CapturedPhoto, type PermissionStatus } from "./shared";

function decodePhoto(value: unknown): CapturedPhoto {
  if (typeof value !== "object" || value === null
    || !("source" in value) || typeof value.source !== "string"
    || !("width" in value) || typeof value.width !== "number" || !Number.isSafeInteger(value.width) || value.width <= 0
    || !("height" in value) || typeof value.height !== "number" || !Number.isSafeInteger(value.height) || value.height <= 0
    || !("mimeType" in value) || value.mimeType !== "image/jpeg"
    || !("capturedAtMs" in value) || typeof value.capturedAtMs !== "number" || !Number.isSafeInteger(value.capturedAtMs)) throw new NativeError("protocol", "Invalid captured photo");
  if (!("file" in value) || typeof value.file !== "object" || value.file === null
    || !("uri" in value.file) || typeof value.file.uri !== "string"
    || !("source" in value.file) || typeof value.file.source !== "string"
    || !("name" in value.file) || typeof value.file.name !== "string"
    || !("size" in value.file) || typeof value.file.size !== "number" || !Number.isSafeInteger(value.file.size) || value.file.size < 0
    || !("mimeType" in value.file) || value.file.mimeType !== "image/jpeg") throw new NativeError("protocol", "Invalid captured file");
  if (!("id" in value.file) || typeof value.file.id !== "string" || !("src" in value.file) || typeof value.file.src !== "string") throw new NativeError("protocol", "Invalid managed photo");
  const file: CapturedFile = { id: value.file.id, src: value.file.src, width: value.width, height: value.height, uri: value.file.uri, source: value.file.source, name: value.file.name, size: value.file.size, mimeType: value.file.mimeType };
  return { file, source: value.source, width: value.width, height: value.height, mimeType: value.mimeType, capturedAt: value.capturedAtMs };
}

export function useCamera({ facing = "back" }: { facing?: "back" | "front" } = {}) {
  return useSession("photo", decodePhoto, JSON.stringify({ facing }));
}
