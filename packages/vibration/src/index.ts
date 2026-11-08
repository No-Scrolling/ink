import { callNative } from "ink/native";

export type VibrationSegment = { ms: number; strength: number };
export type VibrationOptions = { alarm?: boolean };

export function vibrate(
  ms: number,
  options?: VibrationOptions & { strength?: number },
): Promise<void>;
export function vibrate(
  pattern: readonly VibrationSegment[],
  options?: VibrationOptions,
): Promise<void>;
export async function vibrate(
  input: number | readonly VibrationSegment[],
  { strength, alarm = false }: VibrationOptions & { strength?: number } = {},
): Promise<void> {
  const segments = typeof input === "number" ? [{ ms: input, strength }] : input;
  if (!Array.isArray(segments) || segments.length === 0)
    throw new RangeError("Vibration needs at least one segment");
  for (const segment of segments) {
    if (!segment || !Number.isSafeInteger(segment.ms) || segment.ms < 0) {
      throw new RangeError("Vibration durations must be non-negative integer milliseconds");
    }
    if (
      (segment.strength !== undefined &&
        (!Number.isFinite(segment.strength) || segment.strength < 0 || segment.strength > 1)) ||
      (typeof input !== "number" && segment.strength === undefined)
    ) {
      throw new RangeError("Vibration strength must be between 0 and 1");
    }
  }
  await callNative("vibration", "vibrate", { segments, alarm });
}

export async function stopVibrating(): Promise<void> {
  await callNative("vibration", "stop", {});
}
