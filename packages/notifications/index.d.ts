type ResourceErrorKind =
  | "unavailable"
  | "permission-denied"
  | "permission-blocked"
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

export type NotificationPermission = AsyncResource<
  "granted" | "denied" | "blocked"
> & {
  request(): void;
};

interface LocalNotificationBase {
  readonly id: string;
  readonly title: string;
  readonly body: string;
  readonly href?: string;
  readonly data?: string;
}

export type LocalNotification = LocalNotificationBase &
  (
    | { readonly delayMs: number; readonly triggerAtMs?: never }
    | { readonly triggerAtMs: number; readonly delayMs?: never }
  );

export type NotificationOperationErrorKind =
  | "permission-denied"
  | "permission-blocked"
  | "invalid-request"
  | "capacity"
  | "storage"
  | "unavailable"
  | "timeout"
  | "protocol"
  | "unexpected";

interface LocalNotificationActions {
  schedule(notification: LocalNotification): void;
  cancel(id: string): void;
}

export type LocalNotifications = LocalNotificationActions &
  (
    | { readonly status: "idle" }
    | {
        readonly status: "error";
        readonly operation: "schedule" | "cancel";
        readonly id: string;
        readonly errorKind: NotificationOperationErrorKind;
        readonly errorMessage: string;
        readonly errorRetryable: boolean;
      }
  );

export type NativeEvent<T> =
  | { readonly status: "empty" }
  | { readonly status: "ready"; readonly value: T; consume(): void };

export interface NotificationTap {
  readonly id: string;
  readonly data: string;
}

export declare function notificationPermission(): NotificationPermission;
export declare function localNotifications(): LocalNotifications;
export declare function notificationTap(): NativeEvent<NotificationTap>;
