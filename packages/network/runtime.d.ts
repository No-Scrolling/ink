import type * as Streams from "web-streams-polyfill";
import type * as URLs from "whatwg-url";
import type * as Bodies from "./src/blob";
import type * as HTTP from "./src/fetch";
import type * as Sockets from "./src/websocket";

declare global {
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
