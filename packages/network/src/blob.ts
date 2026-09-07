import { URLSearchParams } from "whatwg-url";
import { ReadableStream } from "web-streams-polyfill";
import { callNative } from "ink/native";
import { fromByteArray, toByteArray } from "base64-js";

type BlobPart = string | ArrayBuffer | ArrayBufferView | Blob;
type NativePart = { src: string; offset: number; size: number };
type Part = Uint8Array | NativePart;
const partSize = (part: Part) => part instanceof Uint8Array ? part.length : part.size;
export class Blob {
  readonly size: number;
  readonly type: string;
  #parts: Part[];
  constructor(parts: Iterable<BlobPart> = [], options: { type?: string; endings?: "transparent" | "native" } = {}) {
    this.#parts = [];
    for (const part of parts) {
      if (part instanceof Blob) this.#parts.push(...part.#parts);
      else if (part instanceof ArrayBuffer) this.#parts.push(new Uint8Array(part.slice(0)));
      else if (ArrayBuffer.isView(part)) this.#parts.push(new Uint8Array(part.buffer, part.byteOffset, part.byteLength).slice());
      else this.#parts.push(new TextEncoder().encode(options.endings === "native" ? String(part).replace(/\r\n|\r/g, "\n") : String(part)));
    }
    this.size = this.#parts.reduce((size, part) => size + partSize(part), 0);
    const type = String(options.type ?? "");
    this.type = /[^\x20-\x7e]/.test(type) ? "" : type.toLowerCase();
  }
  static fromNative(src: string, size: number, type: string): Blob {
    const blob = new Blob([], { type });
    blob.#parts = [{ src, offset: 0, size }];
    Object.defineProperty(blob, "size", { value: size });
    return blob;
  }
  nativeParts(): Iterable<{ bytes: string } | NativePart> | undefined {
    if (!this.#parts.some(part => !(part instanceof Uint8Array))) return undefined;
    const parts = this.#parts;
    return (function* () {
      for (const part of parts) {
        if (part instanceof Uint8Array) {
          for (let offset = 0; offset < part.length; offset += 32768) yield { bytes: fromByteArray(part.subarray(offset, offset + 32768)) };
        } else yield part;
      }
    })();
  }
  slice(start = 0, end = this.size, type = "") {
    const normalise = (value: number) => value < 0 ? Math.max(this.size + Math.trunc(value), 0) : Math.min(Math.trunc(value) || 0, this.size);
    let offset = 0;
    const result = new Blob([], { type });
    const from = normalise(start), to = normalise(end);
    for (const part of this.#parts) {
      const left = Math.max(0, from - offset), right = Math.min(partSize(part), to - offset);
      if (right > left) {
        result.#parts.push(part instanceof Uint8Array ? part.subarray(left, right) : { ...part, offset: part.offset + left, size: right - left });
      }
      offset += partSize(part);
    }
    Object.defineProperty(result, "size", { value: Math.max(to - from, 0) });
    return result;
  }
  async bytes() {
    const bytes = new Uint8Array(this.size);
    let offset = 0;
    const reader = this.stream().getReader();
    try {
      for (;;) { const chunk = await reader.read(); if (chunk.done) break; bytes.set(chunk.value, offset); offset += chunk.value.length; }
    } finally { reader.releaseLock(); }
    return bytes;
  }
  async arrayBuffer() { return (await this.bytes()).buffer; }
  async text() { return new TextDecoder().decode(await this.bytes()); }
  stream(): ReadableStream<Uint8Array> {
    let index = 0, offset = 0;
    const parts = this.#parts;
    return new ReadableStream({
      async pull(controller) {
        while (index < parts.length && offset === partSize(parts[index])) { index++; offset = 0; }
        if (index === parts.length) { controller.close(); return; }
        const part = parts[index];
        const end = Math.min(offset + 32 * 1024, partSize(part));
        const bytes = part instanceof Uint8Array ? part.slice(offset, end)
          : toByteArray(await callNative("network", "stream-file-read", { src: part.src, offset: part.offset + offset, size: end - offset }));
        if (bytes.length !== end - offset) throw new TypeError("Managed file changed while reading");
        controller.enqueue(bytes);
        offset = end;
      },
    }, { highWaterMark: 0 });
  }
  get [Symbol.toStringTag]() { return "Blob"; }
}
export class File extends Blob {
  readonly name: string;
  readonly lastModified: number;
  readonly webkitRelativePath = "";
  constructor(parts: Iterable<BlobPart>, name: string, options: { type?: string; lastModified?: number } = {}) {
    super(parts, options);
    this.name = String(name);
    this.lastModified = options.lastModified ?? Date.now();
  }
  get [Symbol.toStringTag]() { return "File"; }
}

type FormValue = string | File;
export class FormData implements Iterable<[string, FormValue]> {
  #entries: [string, FormValue][] = [];
  append(name: string, value: string | Blob, filename?: string) {
    this.#entries.push([String(name), value instanceof Blob
      ? new File([value], filename ?? (value instanceof File ? value.name : "blob"), { type: value.type, lastModified: value instanceof File ? value.lastModified : undefined })
      : String(value)]);
  }
  delete(name: string) { this.#entries = this.#entries.filter(entry => entry[0] !== String(name)); }
  get(name: string): FormValue | null { return this.#entries.find(entry => entry[0] === String(name))?.[1] ?? null; }
  getAll(name: string): FormValue[] { return this.#entries.filter(entry => entry[0] === String(name)).map(entry => entry[1]); }
  has(name: string) { return this.#entries.some(entry => entry[0] === String(name)); }
  set(name: string, value: string | Blob, filename?: string) {
    const index = this.#entries.findIndex(entry => entry[0] === String(name));
    this.delete(name);
    this.append(name, value, filename);
    const entry = this.#entries.pop();
    if (entry) this.#entries.splice(index < 0 ? this.#entries.length : index, 0, entry);
  }
  *entries(): IterableIterator<[string, FormValue]> { yield* this.#entries; }
  *keys() { for (const [name] of this.#entries) yield name; }
  *values() { for (const [, value] of this.#entries) yield value; }
  [Symbol.iterator]() { return this.entries(); }
  forEach(callback: (value: FormValue, name: string, form: FormData) => void, thisArg?: unknown) {
    for (const [name, value] of this) callback.call(thisArg, value, name, this);
  }
  get [Symbol.toStringTag]() { return "FormData"; }
}

export function multipart(form: FormData): Blob {
  const boundary = `----ink-${Math.random().toString(36).slice(2)}-${Date.now().toString(36)}`;
  const parts: BlobPart[] = [];
  const escape = (text: string) => text.replace(/\r\n|\r|\n/g, "\r\n").replace(/\r/g, "%0D").replace(/\n/g, "%0A").replace(/"/g, "%22");
  for (const [name, value] of form) {
    let header = `--${boundary}\r\nContent-Disposition: form-data; name="${escape(name)}"`;
    if (value instanceof File) {
      header += `; filename="${escape(value.name)}"\r\nContent-Type: ${value.type || "application/octet-stream"}`;
    }
    parts.push(`${header}\r\n\r\n`, typeof value === "string" ? value.replace(/\r\n|\r|\n/g, "\r\n") : value, "\r\n");
  }
  parts.push(`--${boundary}--\r\n`);
  return new Blob(parts, { type: `multipart/form-data; boundary=${boundary}` });
}

export function parseFormData(bytes: Uint8Array, contentType: string): FormData {
  const form = new FormData();
  if (/^application\/x-www-form-urlencoded(?:;|$)/i.test(contentType)) {
    for (const [name, value] of new URLSearchParams(new TextDecoder().decode(bytes))) form.append(name, value);
    return form;
  }
  const boundary = /^multipart\/form-data\s*;.*?boundary=(?:"([^"]+)"|([^;\s]+))/i.exec(contentType);
  if (!boundary) throw new TypeError("Response is not form data");
  const encoder = new TextEncoder();
  const delimiter = encoder.encode(`\r\n--${boundary[1] ?? boundary[2]}`);
  const find = (needle: Uint8Array, start: number) => {
    outer: for (let index = start; index <= bytes.length - needle.length; index++) {
      for (let offset = 0; offset < needle.length; offset++) if (bytes[index + offset] !== needle[offset]) continue outer;
      return index;
    }
    return -1;
  };
  let offset = find(delimiter.subarray(2), 0);
  if (offset < 0) throw new TypeError("Invalid multipart boundary");
  offset += delimiter.length - 2;
  while (bytes[offset] !== 45 || bytes[offset + 1] !== 45) {
    if (bytes[offset] !== 13 || bytes[offset + 1] !== 10) throw new TypeError("Invalid multipart framing");
    const headersEnd = find(encoder.encode("\r\n\r\n"), offset + 2);
    if (headersEnd < 0 || headersEnd - offset > 16 * 1024) throw new TypeError("Invalid multipart headers");
    const headers = new TextDecoder().decode(bytes.subarray(offset + 2, headersEnd));
    const disposition = /^content-disposition:\s*form-data;(.+)$/im.exec(headers)?.[1];
    const name = disposition && /(?:^|;)\s*name="([^"]*)"/i.exec(disposition)?.[1];
    const filename = disposition && /(?:^|;)\s*filename="([^"]*)"/i.exec(disposition)?.[1];
    const end = find(delimiter, headersEnd + 4);
    if (name === undefined || name === null || end < 0) throw new TypeError("Invalid multipart field");
    const value = bytes.subarray(headersEnd + 4, end);
    if (filename !== undefined && filename !== null) {
      const type = /^content-type:\s*([^\r\n]+)/im.exec(headers)?.[1] ?? "application/octet-stream";
      form.append(name, new File([value], filename, { type }));
    } else form.append(name, new TextDecoder().decode(value));
    offset = end + delimiter.length;
  }
  return form;
}
