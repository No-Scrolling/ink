// TODO: Import these from "ink" once local file packages resolve sibling types correctly.
export type ResourceErrorKind =
  | "unavailable"
  | "permission-denied"
  | "permission-blocked"
  | "location-disabled"
  | "nfc-disabled"
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

export declare function openDialler(phoneNumber: string): void;

export type RingtoneKind = "ringtone" | "notification" | "alarm";

export type RingtoneErrorKind =
  | ""
  | "source"
  | "unavailable"
  | "protocol"
  | "unexpected";

export interface RingtoneInstaller {
  readonly status: "idle" | "installing" | "installed" | "error";
  readonly errorKind: RingtoneErrorKind;
  readonly errorMessage: string;
  readonly errorRetryable: boolean;
  set(source: string, kind?: RingtoneKind): void;
}

export interface LightPushMessage {
  readonly id: string;
  readonly groupKey: string;
  readonly title: string;
  readonly body: string;
  readonly route: string;
  readonly receivedAtMs: number;
}

export type LightPushErrorKind =
  | ""
  | "no-distributor"
  | "registration"
  | "subscription"
  | "protocol"
  | "storage"
  | "unexpected";

export interface LightPush {
  readonly status:
    | "idle"
    | "registering"
    | "synchronising"
    | "ready"
    | "error";
  readonly endpoint: string;
  readonly registeredAtMs: number;
  readonly openedKey: string;
  readonly messages: ReadonlyArray<LightPushMessage>;
  readonly errorKind: LightPushErrorKind;
  readonly errorMessage: string;
  readonly errorRetryable: boolean;
  register(subscriptionBaseUrl: string, bearerToken?: string): void;
  retry(): void;
  unregister(): void;
  dismiss(groupKey: string): void;
  clear(): void;
}

export declare function ringtoneInstaller(): RingtoneInstaller;
export declare function lightPush(): LightPush;
