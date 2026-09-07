import { ReadableStream, type ReadableStreamDefaultReader } from "web-streams-polyfill";
import { Blob, FormData, multipart, parseFormData } from "./blob";
import { URL, URLSearchParams } from "whatwg-url";
import { requestHttp } from "./http";
import { callNative } from "ink/native";

const immutableHeaders = new WeakSet<Headers>();
const token = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;
type HeaderInput = Headers | Iterable<readonly [string, string]> | Record<string, string>;
function headerName(value: string): string {
  const name = String(value);
  if (!token.test(name)) throw new TypeError("Invalid HTTP header name");
  return name.toLowerCase();
}
function headerValue(value: string): string {
  const text = String(value).replace(/^[\t ]+|[\t ]+$/g, "");
  if (/[\u0000\r\n\u0100-\uffff]/.test(text)) throw new TypeError("Invalid HTTP header value");
  return text;
}
export class Headers implements Iterable<[string, string]> {
  #values = new Map<string, string[]>();
  constructor(input?: HeaderInput) {
    if (input === undefined) return;
    if (Symbol.iterator in input) {
      for (const pair of input) {
        if (pair.length !== 2) throw new TypeError("Headers require name/value pairs");
        this.append(pair[0], pair[1]);
      }
    } else {
      for (const [name, value] of Object.entries(input)) this.append(name, value);
    }
  }
  #mutable() { if (immutableHeaders.has(this)) throw new TypeError("Headers are immutable"); }
  append(name: string, value: string) {
    this.#mutable();
    const key = headerName(name);
    const text = headerValue(value);
    this.#values.set(key, [...(this.#values.get(key) ?? []), text]);
  }
  set(name: string, value: string) { this.#mutable(); this.#values.set(headerName(name), [headerValue(value)]); }
  delete(name: string) { this.#mutable(); this.#values.delete(headerName(name)); }
  get(name: string): string | null { return this.#values.get(headerName(name))?.join(", ") ?? null; }
  has(name: string) { return this.#values.has(headerName(name)); }
  getSetCookie() { return [...(this.#values.get("set-cookie") ?? [])]; }
  *entries(): IterableIterator<[string, string]> {
    for (const name of [...this.#values.keys()].sort()) {
      if (name === "set-cookie") { for (const value of this.getSetCookie()) yield [name, value]; }
      else yield [name, this.get(name) ?? ""];
    }
  }
  *keys() { for (const [name] of this) yield name; }
  *values() { for (const [, value] of this) yield value; }
  [Symbol.iterator]() { return this.entries(); }
  forEach(callback: (value: string, name: string, headers: Headers) => void, thisArg?: unknown) {
    for (const [name, value] of this) callback.call(thisArg, value, name, this);
  }
}

type BodyInput = string | ArrayBuffer | ArrayBufferView | URLSearchParams | Blob | FormData | ReadableStream<Uint8Array>;
function bodyStream(input: BodyInput | null | undefined): ReadableStream<Uint8Array> | null {
  if (input == null) return null;
  if (input instanceof ReadableStream) return input;
  if (input instanceof FormData) return multipart(input).stream();
  if (input instanceof Blob) return input.stream();
  return new Blob([input instanceof URLSearchParams ? String(input) : input]).stream();
}
class Body {
  protected data: ReadableStream<Uint8Array> | null;
  protected bodySignal?: AbortSignal;
  protected nativeBlob?: Blob;
  #disturbed = new WeakSet<ReadableStream<Uint8Array>>();
  protected get contentType(): string { return ""; }
  constructor(input?: BodyInput | null, signal?: AbortSignal) { this.data = this.track(bodyStream(input)); this.bodySignal = signal; this.nativeBlob = input instanceof Blob ? input : undefined; }
  protected track(source: ReadableStream<Uint8Array> | null): ReadableStream<Uint8Array> | null {
    if (!source) return null;
    const disturbed = this.#disturbed;
    let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
    const stream = new ReadableStream<Uint8Array>({
      async pull(controller) {
        disturbed.add(stream);
        reader ??= source.getReader();
        try {
          const chunk = await reader.read();
          if (chunk.done) { reader.releaseLock(); controller.close(); }
          else controller.enqueue(chunk.value);
        } catch (error) { reader.releaseLock(); controller.error(error); }
      },
      cancel(reason) { disturbed.add(stream); return reader ? reader.cancel(reason) : source.cancel(reason); },
    }, { highWaterMark: 0 });
    return stream;
  }
  get body() { return this.data; }
  get bodyUsed() { return this.data !== null && this.#disturbed.has(this.data); }
  protected copyBody() {
    if (this.bodyUsed || this.data?.locked) throw new TypeError("HTTP body has already been consumed");
    if (!this.data) return null;
    const [original, copy] = this.data.tee();
    this.data = this.track(original);
    return copy;
  }
  protected transferBody() {
    if (this.bodyUsed || this.data?.locked) throw new TypeError("HTTP body has already been consumed");
    if (this.data !== null) this.#disturbed.add(this.data);
  }
  protected takeBody(): ReadableStream<Uint8Array> | null {
    this.transferBody();
    if (!this.data) return null;
    const reader = this.data.getReader();
    return new ReadableStream<Uint8Array>({
      async pull(controller) {
        try {
          const chunk = await reader.read();
          if (chunk.done) { reader.releaseLock(); controller.close(); }
          else controller.enqueue(chunk.value);
        } catch (error) { reader.releaseLock(); controller.error(error); }
      },
      async cancel(reason) { try { await reader.cancel(reason); } finally { reader.releaseLock(); } },
    }, { highWaterMark: 0 });
  }
  async bytes(): Promise<Uint8Array> {
    this.bodySignal?.throwIfAborted();
    this.transferBody();
    if (!this.data) return new Uint8Array();
    const reader = this.data.getReader();
    const chunks: Uint8Array[] = [];
    let size = 0;
    try {
      for (;;) {
        this.bodySignal?.throwIfAborted();
        const { value, done } = await reader.read();
        if (done) break;
        size += value.byteLength;
        if (size > 16 * 1024 * 1024) throw new RangeError("Buffered HTTP body exceeds 16 MiB; consume response.body incrementally");
        chunks.push(value);
      }
    } catch (error) { await reader.cancel(error).catch(() => {}); throw error; }
    finally { reader.releaseLock(); }
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
    return bytes;
  }
  async arrayBuffer(): Promise<ArrayBuffer> { return new Uint8Array(await this.bytes()).buffer; }
  async text(): Promise<string> { return new TextDecoder().decode(await this.bytes()); }
  async json(): Promise<unknown> { return JSON.parse(await this.text()); }
  async blob() {
    if (this.nativeBlob) { this.bodySignal?.throwIfAborted(); this.transferBody(); return this.nativeBlob; }
    return new Blob([await this.bytes()], { type: this.contentType });
  }
  async formData() { return parseFormData(await this.bytes(), this.contentType); }
}
function setContentType(headers: Headers, body: BodyInput | null | undefined) {
  if (headers.has("content-type")) return;
  if (typeof body === "string") headers.set("content-type", "text/plain;charset=UTF-8");
  else if (body instanceof Blob && body.type) headers.set("content-type", body.type);
  else if (body instanceof URLSearchParams) headers.set("content-type", "application/x-www-form-urlencoded;charset=UTF-8");
}
type Init = {
  method?: string; headers?: HeaderInput; body?: BodyInput | null;
  signal?: AbortSignal | null; redirect?: "follow" | "error" | "manual";
  credentials?: "omit" | "same-origin" | "include";
};
export class Request extends Body {
  readonly url: string;
  readonly method: string;
  readonly headers: Headers;
  readonly signal: AbortSignal;
  readonly redirect: "follow" | "error" | "manual";
  readonly credentials: "omit" | "same-origin" | "include";
  #replay: Blob | undefined;
  constructor(input: string | URL | Request, init: Init = {}) {
    const original = input instanceof Request ? input : undefined;
    const supplied = init.body ?? original?.body;
    const body = supplied instanceof FormData ? multipart(supplied) : supplied;
    const signal = init.signal ?? original?.signal ?? new AbortController().signal;
    super(body, signal);
    this.#replay = body instanceof ReadableStream ? (init.body == null ? (original ? original.#replay : undefined) : undefined)
      : body == null ? undefined : body instanceof Blob ? body : new Blob([body instanceof URLSearchParams ? String(body) : body]);
    const url = new URL(original?.url ?? String(input));
    if (url.username || url.password) throw new TypeError("HTTP URLs cannot contain credentials");
    this.url = url.href;
    let method = String(init.method ?? original?.method ?? "GET");
    if (!token.test(method) || ["CONNECT", "TRACE", "TRACK"].includes(method.toUpperCase())) throw new TypeError("Invalid HTTP method");
    if (["DELETE", "GET", "HEAD", "OPTIONS", "POST", "PUT"].includes(method.toUpperCase())) method = method.toUpperCase();
    this.method = method;
    if ((method === "GET" || method === "HEAD") && this.data !== null) throw new TypeError("GET and HEAD requests cannot have a body");
    this.headers = new Headers(init.headers ?? original?.headers);
    setContentType(this.headers, body);
    this.signal = signal;
    this.redirect = init.redirect ?? original?.redirect ?? "follow";
    if (!["follow", "error", "manual"].includes(this.redirect)) throw new TypeError("Invalid redirect mode");
    this.credentials = init.credentials ?? original?.credentials ?? "same-origin";
    if (!["omit", "same-origin", "include"].includes(this.credentials)) throw new TypeError("Invalid credentials mode");
    if (init.body == null && original) this.data = this.track(original.takeBody());
  }
  protected get contentType() { return this.headers.get("content-type") ?? ""; }
  clone() {
    const copy = new Request(this, { body: this.copyBody() });
    copy.#replay = this.#replay;
    return copy;
  }
  replayBody() { return this.#replay?.stream() ?? null; }
  nativeParts() { return this.#replay?.nativeParts(); }
}
type ResponseOptions = { status?: number; statusText?: string; headers?: HeaderInput; url?: string; redirected?: boolean; signal?: AbortSignal };
export class Response extends Body {
  readonly status: number;
  readonly statusText: string;
  readonly headers: Headers;
  readonly url: string;
  readonly redirected: boolean;
  readonly type = "default";
  constructor(body?: BodyInput | null, init: ResponseOptions = {}) {
    body = body instanceof FormData ? multipart(body) : body;
    super(body, init.signal);
    this.status = init.status ?? 200;
    if (!Number.isInteger(this.status) || this.status < 200 || this.status > 599) throw new RangeError("Invalid HTTP response status");
    if ([204, 205, 304].includes(this.status) && this.data !== null) throw new TypeError("This HTTP status cannot have a body");
    this.statusText = init.statusText ?? "";
    if (/[\r\n\u0100-\uffff]/.test(this.statusText)) throw new TypeError("Invalid HTTP status text");
    this.headers = new Headers(init.headers);
    setContentType(this.headers, body);
    this.url = init.url ?? "";
    this.redirected = init.redirected ?? false;
  }
  protected get contentType() { return this.headers.get("content-type") ?? ""; }
  get ok() { return this.status >= 200 && this.status <= 299; }
  clone() {
    const result = new Response(this.copyBody(), { status: this.status, statusText: this.statusText, headers: this.headers, url: this.url, redirected: this.redirected, signal: this.bodySignal });
    Object.defineProperty(result, "type", { value: this.type });
    result.nativeBlob = this.nativeBlob;
    if (immutableHeaders.has(this.headers)) immutableHeaders.add(result.headers);
    return result;
  }
  static json(value: unknown, init: ResponseOptions = {}) {
    const body = JSON.stringify(value);
    if (body === undefined) throw new TypeError("Value cannot be serialised as JSON");
    const headers = new Headers(init.headers);
    if (!headers.has("content-type")) headers.set("content-type", "application/json");
    return new Response(body, { ...init, headers });
  }
  static redirect(url: string, status = 302) {
    if (![301, 302, 303, 307, 308].includes(status)) throw new RangeError("Invalid redirect status");
    const response = new Response(null, { status, headers: { location: new URL(url).href } });
    immutableHeaders.add(response.headers);
    return response;
  }
}

export async function fetch(input: string | URL | Request, init?: Init): Promise<Response> {
  const request = new Request(input, init);
  if (request.url.startsWith("ink-file://")) {
    request.signal.throwIfAborted();
    if (request.method !== "GET" && request.method !== "HEAD") throw new TypeError("Managed files are read-only");
    const file = JSON.parse(await callNative("files", "open", { id: request.url.slice("ink-file://".length) }, { signal: request.signal }));
    if (!file) throw new TypeError("Managed file has been removed");
    return new Response(request.method === "HEAD" ? null : Blob.fromNative(file.src, file.size, file.mimeType), {
      url: request.url, headers: { "content-type": file.mimeType, "content-length": String(file.size) }, signal: request.signal,
    });
  }
  let url = new URL(request.url);
  let method = request.method;
  const headers = new Headers(request.headers);
  let body = request.body;
  for (let redirects = 0; ; redirects++) {
    request.signal.throwIfAborted();
    if (url.protocol !== "https:" && url.protocol !== "file:" && !(url.protocol === "http:" && ["localhost", "127.0.0.1", "[::1]", "10.0.2.2"].includes(url.hostname))) throw new TypeError("Network requests require HTTPS");
    let result;
    try {
      result = await requestHttp({ url: url.href, method, headers: Object.fromEntries(headers), nativeParts: body ? request.nativeParts() : undefined }, body, request.signal);
    } catch (error) {
      request.signal.throwIfAborted();
      throw new TypeError("Network request failed", { cause: error });
    }
    request.signal.throwIfAborted();
    const responseHeaders = new Headers(result.headers);
    const location = responseHeaders.get("location");
    if ([301, 302, 303, 307, 308].includes(result.status) && location !== null && request.redirect !== "manual") {
      await result.body?.cancel();
      if (request.redirect === "error" || redirects >= 20) throw new TypeError("HTTP redirect was not allowed");
      const next = new URL(location, url);
      if (next.username || next.password) throw new TypeError("HTTP redirect contains credentials");
      if (next.origin !== url.origin) {
        for (const name of ["authorization", "proxy-authorization", "cookie", "cookie2", "host"]) headers.delete(name);
      }
      if (([301, 302].includes(result.status) && method === "POST") || (result.status === 303 && method !== "GET" && method !== "HEAD")) {
        method = "GET";
        body = null;
        for (const name of ["content-type", "content-length", "content-encoding", "content-language", "content-location"]) headers.delete(name);
      }
      if (body !== null) {
        body = request.replayBody();
        if (body === null) throw new TypeError("Cannot replay a streaming upload after an HTTP redirect");
      }
      url = next;
      continue;
    }
    const responseBody = method === "HEAD" || [204, 205, 304].includes(result.status) ? null : result.body;
    if (responseBody === null) await result.body?.cancel();
    const response = new Response(responseBody, { status: result.status, statusText: result.statusText, headers: responseHeaders, url: result.url, redirected: redirects > 0, signal: request.signal });
    Object.defineProperty(response, "type", { value: "basic" });
    immutableHeaders.add(response.headers);
    return response;
  }
}
