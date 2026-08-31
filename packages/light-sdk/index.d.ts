import type {
  AsyncResource,
  InkError,
  PermissionResource,
  PermissionStatus,
} from "ink";

export type LightSdkPermissionStatus = PermissionStatus;
export type LightSdkPermission = PermissionResource;

export declare function lightSdkVersion(): AsyncResource<string>;
export declare function lightSdkPermission(permission: "camera"): LightSdkPermission;

export declare function openDialler(phoneNumber: string): void;

export type RingtoneKind = "ringtone" | "notification" | "alarm";

export type RingtoneErrorKind =
  | "source"
  | "unavailable"
  | "protocol"
  | "unexpected";

interface RingtoneActions {
  set(source: string, kind?: RingtoneKind): void;
}

export type RingtoneInstaller = RingtoneActions &
  (
    | { readonly status: "idle" | "installing" | "installed" }
    | {
        readonly status: "error";
        readonly error: InkError<RingtoneErrorKind>;
      }
  );

export interface LightPushMessage {
  readonly id: string;
  readonly groupKey: string;
  readonly title: string;
  readonly body: string;
  readonly route: string;
  readonly receivedAtMs: number;
}

export type LightPushErrorKind =
  | "no-distributor"
  | "registration"
  | "subscription"
  | "protocol"
  | "storage"
  | "unexpected";

interface LightPushValue {
  readonly endpoint: string;
  readonly registeredAtMs: number;
  readonly openedKey: string;
  readonly messages: ReadonlyArray<LightPushMessage>;
}

interface LightPushActions {
  register(subscriptionBaseUrl: string, bearerToken?: string): void;
  retry(): void;
  unregister(): void;
  dismiss(groupKey: string): void;
  clear(): void;
}

export type LightPush = LightPushActions &
  LightPushValue &
  (
    | { readonly status: "idle" | "registering" | "synchronising" | "ready" }
    | {
        readonly status: "error";
        readonly error: InkError<LightPushErrorKind>;
      }
  );

export declare function ringtoneInstaller(): RingtoneInstaller;
export declare function lightPush(): LightPush;
