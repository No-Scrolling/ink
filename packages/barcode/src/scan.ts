import { NativeError } from "ink/native";
import { useSession, codeFormats, type CodeFormat, type CodeScan } from "@ink/camera/internal";
export { camera, CameraPreview, codeFormats, type CodeFormat, type CodeScan, type PermissionStatus } from "@ink/camera/internal";

function codeFormat(value: unknown): CodeFormat {
  const format = codeFormats.find(format => format === value);
  if (!format) throw new NativeError("protocol", "Invalid barcode format");
  return format;
}
function decodeCode(value: unknown): CodeScan {
  if (typeof value !== "object" || value === null || !("text" in value) || typeof value.text !== "string" || !("format" in value)) throw new NativeError("protocol", "Invalid scanned code");
  if (!("rawBytes" in value) || (value.rawBytes !== null && (!Array.isArray(value.rawBytes)
    || !value.rawBytes.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)))) throw new NativeError("protocol", "Invalid barcode bytes");
  return { text: value.text, format: codeFormat(value.format), rawBytes: value.rawBytes === null ? null : new Uint8Array(value.rawBytes) };
}

export function useCodeScanner({ formats = [...codeFormats], continuous = false, intervalMs = 1000, facing = "back" }: { formats?: readonly CodeFormat[]; continuous?: boolean; intervalMs?: number; facing?: "back" | "front" } = {}) {
  if (!Number.isSafeInteger(intervalMs) || intervalMs < 100 || intervalMs > 60_000) throw new RangeError("Scan interval must be between 100 and 60000 milliseconds");
  if (!formats.length) throw new TypeError("Choose at least one barcode format");
  return useSession("scanner", decodeCode, JSON.stringify({ formats: formats.map(codeFormat), continuous, intervalMs, facing }));
}
