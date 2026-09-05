import { callNative, NativeError } from "ink/native";

export interface NfcRecord {
  kind: "text" | "uri" | "binary";
  value: string;
  languageTag: string;
  mimeType: string;
  payloadBase64: string;
}
export interface NfcTag {
  serialNumber: string;
  hasText: boolean;
  text: string;
  hasUri: boolean;
  uri: string;
  records: readonly NfcRecord[];
}

function decodeRecord(value: unknown): NfcRecord {
  if (typeof value !== "object" || value === null
    || !("kind" in value) || (value.kind !== "text" && value.kind !== "uri" && value.kind !== "binary")
    || !("value" in value) || typeof value.value !== "string"
    || !("languageTag" in value) || typeof value.languageTag !== "string"
    || !("mimeType" in value) || typeof value.mimeType !== "string"
    || !("payloadBase64" in value) || typeof value.payloadBase64 !== "string") {
    throw new NativeError("protocol", "Invalid NFC record");
  }
  return { kind: value.kind, value: value.value, languageTag: value.languageTag,
    mimeType: value.mimeType, payloadBase64: value.payloadBase64 };
}

export const nfc = {
  connect, emulate, stopEmulation, handleApdu,
  async read({ signal, timeout = 30_000 }: { signal?: AbortSignal; timeout?: number } = {}): Promise<NfcTag> {
    const value: unknown = JSON.parse(await callNative("nfc", "read", "", { signal, timeoutMs: timeout }));
    if (typeof value !== "object" || value === null
      || !("serialNumber" in value) || typeof value.serialNumber !== "string"
      || !("hasText" in value) || typeof value.hasText !== "boolean"
      || !("text" in value) || typeof value.text !== "string"
      || !("hasUri" in value) || typeof value.hasUri !== "boolean"
      || !("uri" in value) || typeof value.uri !== "string"
      || !("records" in value) || !Array.isArray(value.records)) {
      throw new NativeError("protocol", "Invalid NFC tag");
    }
    return { serialNumber: value.serialNumber, hasText: value.hasText, text: value.text,
      hasUri: value.hasUri, uri: value.uri, records: value.records.map(decodeRecord) };
  },
};

export interface NfcConnection {
  readonly serialNumber: string;
  readonly technology: "iso-dep" | "nfc-a";
  readonly maxTransceiveLength: number;
  transceive(bytes: Uint8Array, options?: { signal?: AbortSignal; timeout?: number }): Promise<Uint8Array>;
  close(): Promise<void>;
}

export async function connect({ technology = "iso-dep", signal, timeout = 30_000 }: {
  technology?: "iso-dep" | "nfc-a"; signal?: AbortSignal; timeout?: number;
} = {}): Promise<NfcConnection> {
  const value: unknown = JSON.parse(await callNative("nfc", "connect", { technology }, { signal, timeoutMs: timeout }));
  if (typeof value !== "object" || value === null
    || !("connection" in value) || typeof value.connection !== "string"
    || !("serialNumber" in value) || typeof value.serialNumber !== "string"
    || !("technology" in value) || (value.technology !== "iso-dep" && value.technology !== "nfc-a")
    || !("maxTransceiveLength" in value) || typeof value.maxTransceiveLength !== "number" || !Number.isSafeInteger(value.maxTransceiveLength) || value.maxTransceiveLength <= 0) {
    throw new NativeError("protocol", "Invalid NFC connection");
  }
  const connection = value.connection;
  const maxTransceiveLength = value.maxTransceiveLength;
  return {
    serialNumber: value.serialNumber, technology: value.technology, maxTransceiveLength: value.maxTransceiveLength,
    async transceive(bytes, { signal, timeout = 5000 } = {}) {
      if (!Number.isInteger(timeout) || timeout < 1 || timeout > 10_000) throw new RangeError("NFC exchange timeout must be 1–10000 milliseconds");
      if (bytes.length < 1 || bytes.length > maxTransceiveLength) throw new RangeError("NFC command exceeds the connection limit");
      const result: unknown = JSON.parse(await callNative("nfc", "transceive", { connection, bytes: Array.from(bytes), timeout }, { signal, timeoutMs: timeout + 1000 }));
      if (!Array.isArray(result) || !result.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)) {
        throw new NativeError("protocol", "Invalid NFC response");
      }
      return new Uint8Array(result);
    },
    async close() { await callNative("nfc", "close", { connection }); },
  };
}

export async function emulate({ aids, responses = [], fallback = "6D00", deadline = 500 }: {
  aids: readonly string[];
  responses?: readonly { command: string; response: string }[];
  fallback?: string;
  deadline?: number;
}): Promise<void> {
  await callNative("nfc", "emulate", { aids, responses, fallback, deadline });
}

export async function stopEmulation(): Promise<void> { await callNative("nfc", "stop-emulation", {}); }

let handlingApdu = false;

export async function handleApdu(handler: (command: Uint8Array) => Uint8Array | Promise<Uint8Array>, { signal }: { signal: AbortSignal }): Promise<void> {
  if (handlingApdu) throw new NativeError("unavailable", "An APDU handler is already running");
  handlingApdu = true;
  try {
    while (!signal.aborted) {
      let value: unknown;
      try {
        value = JSON.parse(await callNative("nfc", "apdu-next", {}, { signal, timeoutMs: 60_000 }));
      } catch (error) {
        if (error instanceof NativeError && error.kind === "timeout" && !signal.aborted) continue;
        throw error;
      }
      if (typeof value !== "object" || value === null || !("id" in value) || typeof value.id !== "number"
        || !("command" in value) || typeof value.command !== "string" || !/^(?:[0-9A-F]{2})+$/.test(value.command)) {
        throw new NativeError("protocol", "Invalid APDU request");
      }
      const hex = value.command;
      const command = Uint8Array.from({ length: hex.length / 2 }, (_, index) => parseInt(hex.slice(index * 2, index * 2 + 2), 16));
      const response = await handler(command);
      if (signal.aborted) break;
      try {
        await callNative("nfc", "apdu-response", { id: value.id, response: Array.from(response, byte => byte.toString(16).padStart(2, "0")).join("") }, { signal });
      } catch (error) {
        if (!(error instanceof NativeError) || error.kind !== "unavailable") throw error;
      }
    }
  } finally {
    try { await callNative("nfc", "apdu-stop", {}); }
    finally { handlingApdu = false; }
  }
}
