import type * as Streams from "web-streams-polyfill";
import type * as URLs from "whatwg-url";
import type * as Bodies from "./src/blob";
import type * as HTTP from "./src/fetch";
import type * as Sockets from "./src/websocket";

declare global {
  function setTimeout<T extends unknown[]>(callback: (...args: T) => void, delay?: number, ...args: T): number;
  function setInterval<T extends unknown[]>(callback: (...args: T) => void, delay?: number, ...args: T): number;
  function clearTimeout(id?: number): void;
  function clearInterval(id?: number): void;
  function queueMicrotask(callback: () => void): void;
  const performance: { now(): number; readonly timeOrigin: number };
  const console: { log(...values: unknown[]): void; info(...values: unknown[]): void; warn(...values: unknown[]): void; error(...values: unknown[]): void; debug(...values: unknown[]): void };

  class TextEncoder {
    readonly encoding: "utf-8";
    encode(input?: string): Uint8Array<ArrayBuffer>;
    encodeInto(input: string, destination: Uint8Array): { read: number; written: number };
  }
  class TextDecoder {
    constructor(label?: string, options?: { fatal?: boolean; ignoreBOM?: boolean });
    readonly encoding: "utf-8";
    readonly fatal: boolean;
    readonly ignoreBOM: boolean;
    decode(input?: ArrayBuffer | ArrayBufferView, options?: { stream?: boolean }): string;
  }
  class DOMException extends Error {
    constructor(message?: string, name?: string);
    readonly code: number;
  }
  interface EventInit { bubbles?: boolean; cancelable?: boolean; composed?: boolean }
  class Event {
    constructor(type: string, options?: EventInit);
    readonly type: string;
    readonly bubbles: boolean;
    readonly cancelable: boolean;
    readonly composed: boolean;
    readonly target: EventTarget | null;
    readonly currentTarget: EventTarget | null;
    readonly eventPhase: number;
    readonly defaultPrevented: boolean;
    readonly timeStamp: number;
    readonly isTrusted: false;
    cancelBubble: boolean;
    returnValue: boolean;
    stopPropagation(): void;
    stopImmediatePropagation(): void;
    preventDefault(): void;
    composedPath(): EventTarget[];
    static readonly NONE: 0; static readonly CAPTURING_PHASE: 1; static readonly AT_TARGET: 2; static readonly BUBBLING_PHASE: 3;
    readonly NONE: 0; readonly CAPTURING_PHASE: 1; readonly AT_TARGET: 2; readonly BUBBLING_PHASE: 3;
  }
  class CustomEvent<T = unknown> extends Event {
    constructor(type: string, options?: EventInit & { detail?: T });
    readonly detail: T;
  }
  type EventListener = (event: Event) => void;
  interface EventListenerObject { handleEvent(event: Event): void }
  interface EventListenerOptions { capture?: boolean }
  interface AddEventListenerOptions extends EventListenerOptions { once?: boolean; passive?: boolean; signal?: AbortSignal }
  class EventTarget {
    addEventListener(type: string, listener: EventListener | EventListenerObject | null, options?: boolean | AddEventListenerOptions): void;
    removeEventListener(type: string, listener: EventListener | EventListenerObject | null, options?: boolean | EventListenerOptions): void;
    dispatchEvent(event: Event): boolean;
  }
  class AbortSignal extends EventTarget {
    private constructor();
    readonly aborted: boolean;
    readonly reason: unknown;
    onabort: ((event: Event) => void) | null;
    throwIfAborted(): void;
    static abort(reason?: unknown): AbortSignal;
    static timeout(milliseconds: number): AbortSignal;
    static any(signals: Iterable<AbortSignal>): AbortSignal;
  }
  class AbortController {
    readonly signal: AbortSignal;
    abort(reason?: unknown): void;
  }

  const URL: typeof URLs.URL;
  type URL = URLs.URL;
  const URLSearchParams: typeof URLs.URLSearchParams;
  type URLSearchParams = URLs.URLSearchParams;
  const Blob: typeof Bodies.Blob;
  interface Blob extends Bodies.Blob {}
  const File: typeof Bodies.File;
  type File = Bodies.File;
  const FormData: typeof Bodies.FormData;
  interface FormData extends Bodies.FormData {}
  type BlobPart = ConstructorParameters<typeof Blob>[0] extends Iterable<infer P> | undefined ? P : never;
  const Headers: typeof HTTP.Headers;
  type Headers = HTTP.Headers;
  const Request: typeof HTTP.Request;
  type Request = HTTP.Request;
  const Response: typeof HTTP.Response;
  type Response = HTTP.Response;
  const fetch: typeof HTTP.fetch;
  type RequestInit = NonNullable<ConstructorParameters<typeof Request>[1]>;
  type RequestInfo = Parameters<typeof fetch>[0];
  type HeadersInit = NonNullable<ConstructorParameters<typeof Headers>[0]>;
  type ResponseInit = NonNullable<ConstructorParameters<typeof Response>[1]>;
  type BodyInit = NonNullable<ConstructorParameters<typeof Response>[0]>;
  const WebSocket: typeof Sockets.WebSocket;
  type WebSocket = Sockets.WebSocket;
  const MessageEvent: typeof Sockets.MessageEvent;
  type MessageEvent = Sockets.MessageEvent;
  const CloseEvent: typeof Sockets.CloseEvent;
  type CloseEvent = Sockets.CloseEvent;

  const ReadableStream: typeof Streams.ReadableStream;
  type ReadableStream<R = unknown> = Streams.ReadableStream<R>;
  const WritableStream: typeof Streams.WritableStream;
  type WritableStream<W = unknown> = Streams.WritableStream<W>;
  const TransformStream: typeof Streams.TransformStream;
  type TransformStream<I = unknown, O = unknown> = Streams.TransformStream<I, O>;
  const ByteLengthQueuingStrategy: typeof Streams.ByteLengthQueuingStrategy;
  type ByteLengthQueuingStrategy = Streams.ByteLengthQueuingStrategy;
  const CountQueuingStrategy: typeof Streams.CountQueuingStrategy;
  type CountQueuingStrategy = Streams.CountQueuingStrategy;
  type ReadableStreamDefaultReader<R = unknown> = Streams.ReadableStreamDefaultReader<R>;
  type ReadableStreamDefaultController<R = unknown> = Streams.ReadableStreamDefaultController<R>;
  type WritableStreamDefaultWriter<W = unknown> = Streams.WritableStreamDefaultWriter<W>;
  type WritableStreamDefaultController = Streams.WritableStreamDefaultController;
  type TransformStreamDefaultController<O = unknown> = Streams.TransformStreamDefaultController<O>;
}
