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

export interface NfcTagOptions {
  readonly timeoutMs?: number;
}

export interface NfcRecord {
  readonly kind: "text" | "uri" | "binary";
  readonly value: string;
  readonly languageTag: string;
  readonly mimeType: string;
  readonly payloadBase64: string;
}

export interface NfcTag {
  readonly serialNumber: string;
  readonly hasText: boolean;
  readonly text: string;
  readonly hasUri: boolean;
  readonly uri: string;
  readonly records: ReadonlyArray<NfcRecord>;
}

export declare function nfcTag(
  options?: NfcTagOptions,
): AsyncResource<NfcTag>;
