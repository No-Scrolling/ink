import { NativeError } from "ink/native";
import type { FileRef } from "./index";

export function decodeFileRef(value: string): FileRef | null {
  const file: unknown = JSON.parse(value);
  if (file === null) return null;
  if (typeof file !== "object" || !["id", "src", "name", "mimeType"].every(key => typeof Reflect.get(file, key) === "string")
    || typeof Reflect.get(file, "size") !== "number" || !Number.isSafeInteger(Reflect.get(file, "size")) || Reflect.get(file, "size") < 0) {
    throw new NativeError("protocol", "Invalid managed file");
  }
  for (const key of ["width", "height", "duration"]) {
    const number: unknown = Reflect.get(file, key);
    if (number !== undefined && (typeof number !== "number" || !Number.isFinite(number) || number < 0)) throw new NativeError("protocol", "Invalid media metadata");
  }
  return file as FileRef;
}
