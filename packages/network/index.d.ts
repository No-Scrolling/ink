import type { AsyncResource, ResourceError } from "ink";

// TODO: Add uploads, streaming and WebSockets as apps need them.

export interface JsonOptions {
  readonly query?: Readonly<Record<string, string | number | boolean>>;
  readonly headers?: Readonly<Record<string, string>>;
  readonly timeoutMs?: number;
}

export interface CachedJsonOptions extends JsonOptions {
  /** Fresh responses are reused across launches for this long. Defaults to five minutes. */
  readonly maxAgeMs?: number;
  /** An older response may be shown when refreshing fails. Defaults to one day. */
  readonly staleIfErrorMs?: number;
}

export type CachedResource<T, E> = {
  reload(): void;
} &
  (
    | { readonly status: "loading" }
    | { readonly status: "ready"; readonly value: T; readonly updatedAtMs: number }
    | {
        readonly status: "stale";
        readonly value: T;
        readonly updatedAtMs: number;
        readonly error: E & { readonly attemptedAtMs: number };
      }
    | { readonly status: "error"; readonly error: E }
  );

export interface MutationOptions {
  readonly method: "POST" | "PUT" | "PATCH" | "DELETE";
  readonly query?: Readonly<Record<string, string | number | boolean>>;
  readonly headers?: Readonly<Record<string, string>>;
  readonly body?: Readonly<Record<string, string | number | boolean>>;
  readonly timeoutMs?: number;
}

export type Mutation<T, E> = { run(): void } &
  (
    | { readonly status: "idle" }
    | { readonly status: "running" }
    | { readonly status: "ready"; readonly value: T }
    | { readonly status: "error"; readonly error: E }
  );

export declare function json<T>(
  url: string,
  options?: JsonOptions,
): AsyncResource<T, ResourceError>;

export declare function cachedJson<T>(
  url: string,
  options?: CachedJsonOptions,
): CachedResource<T, ResourceError>;

export declare function mutation<T>(
  url: string,
  options: MutationOptions,
): Mutation<T, ResourceError>;
