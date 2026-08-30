// TODO: Import these from "ink" once local file packages resolve sibling types correctly.
type ResourceErrorKind =
  | "unavailable"
  | "permission-denied"
  | "permission-blocked"
  | "location-disabled"
  | "nfc-disabled"
  | "timeout"
  | "protocol"
  | "unexpected";

interface ResourceError {
  readonly kind: ResourceErrorKind;
  readonly message: string;
  readonly retryable: boolean;
}

type AsyncResource<T> = {
  reload(): void;
} &
  (
    | { readonly status: "loading" }
    | { readonly status: "ready"; readonly value: T }
    | { readonly status: "error"; readonly error: ResourceError }
  );

export type LocationAccuracy = "approximate" | "precise";

export type LocationPermissionStatus =
  | "granted"
  | "denied"
  | "blocked"
  | "unknown";

export type LocationPermission = AsyncResource<LocationPermissionStatus> & {
  request(): void;
};

export interface CurrentLocationOptions {
  readonly accuracy?: LocationAccuracy;
  readonly maxAgeMs?: number;
  readonly timeoutMs?: number;
}

export interface LocationFix {
  readonly latitude: number;
  readonly longitude: number;
  readonly accuracy: number;
  readonly provider: "gps" | "network" | "passive";
  readonly timestamp: number;
}

export declare function locationPermission(
  accuracy?: LocationAccuracy,
): LocationPermission;

export declare function currentLocation(
  options?: CurrentLocationOptions,
): AsyncResource<LocationFix>;
