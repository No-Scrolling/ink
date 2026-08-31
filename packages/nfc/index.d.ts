import type { AsyncResource } from "ink";

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
