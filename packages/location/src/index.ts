import { callNative, NativeError } from "ink/native";

export type LocationAccuracy = "balanced" | "high";
export interface DefaultLocation {
  latitude: number;
  longitude: number;
}

export interface LocationFix {
  latitude: number;
  longitude: number;
  accuracy: number;
  timestamp: number;
  provider: string;
}
export interface LocationOptions {
  accuracy?: LocationAccuracy;
  maximumAge?: number;
  timeout?: number;
  signal?: AbortSignal;
}

export interface LocationWatchOptions {
  accuracy?: LocationAccuracy;
  interval?: number;
  distance?: number;
  signal?: AbortSignal;
}
export interface LocationTrackingState {
  running: boolean;
  fix: LocationFix | null;
  error: string | null;
}

function parseFix(value: unknown): LocationFix {
  if (typeof value !== "object" || value === null
    || !("latitude" in value) || typeof value.latitude !== "number" || !Number.isFinite(value.latitude) || Math.abs(value.latitude) > 90
    || !("longitude" in value) || typeof value.longitude !== "number" || !Number.isFinite(value.longitude) || Math.abs(value.longitude) > 180
    || !("accuracy" in value) || typeof value.accuracy !== "number" || !Number.isFinite(value.accuracy) || value.accuracy < 0
    || !("timestamp" in value) || typeof value.timestamp !== "number" || !Number.isFinite(value.timestamp)
    || !("provider" in value) || typeof value.provider !== "string") {
    throw new NativeError("protocol", "Invalid native location fix");
  }
  return { latitude: value.latitude, longitude: value.longitude, accuracy: value.accuracy, timestamp: value.timestamp, provider: value.provider };
}

function updateOptions(options: LocationWatchOptions) {
  const { accuracy = "balanced", interval = 5_000, distance = 0 } = options;
  if (!Number.isSafeInteger(interval) || interval < 1_000 || interval > 3_600_000) {
    throw new RangeError("Location interval must be between one second and one hour");
  }
  if (!Number.isFinite(distance) || distance < 0 || distance > 100_000) {
    throw new RangeError("Location distance must be between zero and 100,000 metres");
  }
  return { accuracy: accuracy === "high" ? "precise" : "approximate", intervalMs: interval, distanceMetres: distance };
}

function parseTracking(value: unknown): LocationTrackingState {
  if (typeof value !== "object" || value === null || !("running" in value) || typeof value.running !== "boolean"
    || !("fix" in value) || !("error" in value) || value.error !== null && typeof value.error !== "string") {
    throw new NativeError("protocol", "Invalid location tracking state");
  }
  return { running: value.running, fix: value.fix === null ? null : parseFix(value.fix), error: value.error };
}

export const location = {
  async default(): Promise<DefaultLocation | null> {
    const version = await callNative("light-sdk", "version", "");
    if (!/^\d+\.\d+\.\d+$/.test(version)) {
      throw new NativeError("protocol", "Invalid Light SDK version");
    }
    const [major, minor, patch] = version.split(".").map(Number);
    if (major === 0 && (minor < 1 || (minor === 1 && patch < 2))) {
      throw new NativeError("unsupported", "LightOS default location requires Light SDK 0.1.2 or newer");
    }
    const value: unknown = JSON.parse(await callNative("light-sdk", "default-location", ""));
    if (value === null) return null;
    if (typeof value !== "object"
      || !("latitude" in value) || typeof value.latitude !== "number" || !Number.isFinite(value.latitude) || Math.abs(value.latitude) > 90
      || !("longitude" in value) || typeof value.longitude !== "number" || !Number.isFinite(value.longitude) || Math.abs(value.longitude) > 180) {
      throw new NativeError("protocol", "Invalid default location");
    }
    return { latitude: value.latitude, longitude: value.longitude };
  },
  async *watch(options: LocationWatchOptions = {}): AsyncGenerator<LocationFix> {
    const watch = Number(await callNative("location", "watch-start", updateOptions(options), { signal: options.signal }));
    if (!Number.isSafeInteger(watch) || watch <= 0) throw new NativeError("protocol", "Invalid location watch");
    let version = 0;
    try {
      while (!options.signal?.aborted) {
        const value: unknown = JSON.parse(await callNative("location", "watch-next", { watch, version }, { signal: options.signal, timeoutMs: 25_000 }));
        if (value === null) continue;
        if (typeof value !== "object" || !("version" in value) || typeof value.version !== "number"
          || !Number.isSafeInteger(value.version) || value.version <= version || !("fix" in value)) {
          throw new NativeError("protocol", "Invalid location watch update");
        }
        version = value.version;
        yield parseFix(value.fix);
      }
    } finally {
      await callNative("location", "watch-stop", { watch });
    }
  },
  async startTracking(options: Omit<LocationWatchOptions, "signal"> = {}): Promise<LocationTrackingState> {
    return parseTracking(JSON.parse(await callNative("location", "tracking-start", updateOptions(options))));
  },
  async stopTracking(): Promise<void> {
    await callNative("location", "tracking-stop", {});
  },
  async getTracking(): Promise<LocationTrackingState> {
    return parseTracking(JSON.parse(await callNative("location", "tracking-status", {})));
  },
  async current(options: LocationOptions = {}): Promise<LocationFix> {
    const { accuracy = "balanced", maximumAge = 0, timeout = 15_000, signal } = options;
    if (!Number.isSafeInteger(maximumAge) || maximumAge < 0 || maximumAge > 3_600_000) {
      throw new RangeError("Location maximumAge must be between zero and one hour");
    }
    const value: unknown = JSON.parse(await callNative("location", "current", {
      accuracy: accuracy === "high" ? "precise" : "approximate", maxAgeMs: maximumAge,
    }, { timeoutMs: timeout, signal }));
    return parseFix(value);
  },
};
