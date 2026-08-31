import type {
  AsyncResource,
  PermissionResource,
  PermissionStatus,
} from "ink";

export type LocationAccuracy = "approximate" | "precise";

export type LocationPermissionStatus = PermissionStatus;
export type LocationPermission = PermissionResource;

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
