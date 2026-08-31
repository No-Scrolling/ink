import type { InkError } from "ink";

export type BackgroundErrorKind =
  | "unavailable"
  | "timeout"
  | "http"
  | "invalid-data"
  | "storage"
  | "scheduler"
  | "unexpected";

export interface BackgroundError extends InkError<BackgroundErrorKind> {
  readonly attemptedAtMs: number;
}

export type BackgroundResource<T> =
  | { readonly status: "waiting" }
  | { readonly status: "ready"; readonly value: T; readonly updatedAtMs: number }
  | {
      readonly status: "stale";
      readonly value: T;
      readonly updatedAtMs: number;
      readonly error: BackgroundError;
    }
  | { readonly status: "error"; readonly error: BackgroundError };

export interface PeriodicJsonOptions {
  readonly everyMinutes: number;
  readonly query?: Readonly<Record<string, string | number | boolean>>;
  readonly headers?: Readonly<Record<string, string>>;
  readonly timeoutMs?: number;
}

export declare function periodicJson<T>(
  key: string,
  url: string,
  options: PeriodicJsonOptions,
): BackgroundResource<T>;
