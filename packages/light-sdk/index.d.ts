export type ResourceErrorKind =
  | "unavailable"
  | "permission-denied"
  | "timeout"
  | "protocol"
  | "unexpected";

export interface ResourceError {
  readonly kind: ResourceErrorKind;
  readonly message: string;
  readonly retryable: boolean;
}

export type AsyncResource<T> = { reload(): void } & (
  | { readonly status: "loading" }
  | { readonly status: "ready"; readonly value: T }
  | { readonly status: "error"; readonly error: ResourceError }
);

export type LightSdkPermissionStatus =
  | "granted"
  | "denied"
  | "blocked"
  | "unknown";

export type LightSdkPermission = AsyncResource<LightSdkPermissionStatus> & {
  request(): void;
};

export declare function lightSdkVersion(): AsyncResource<string>;
export declare function lightSdkPermission(permission: "camera"): LightSdkPermission;
