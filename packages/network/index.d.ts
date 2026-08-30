// TODO: Import these from "ink" once local file packages resolve sibling types correctly.
// TODO: Add mutations, uploads, streaming, WebSockets and cache policies as apps need them.
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

type AsyncResource<T, E> = {
  reload(): void;
} &
  (
    | { readonly status: "loading" }
    | { readonly status: "ready"; readonly value: T }
    | { readonly status: "error"; readonly error: E }
  );

export interface JsonOptions {
  readonly query?: Readonly<Record<string, string | number | boolean>>;
  readonly headers?: Readonly<Record<string, string>>;
  readonly timeoutMs?: number;
}

export declare function json<T>(
  url: string,
  options?: JsonOptions,
): AsyncResource<T, ResourceError>;
