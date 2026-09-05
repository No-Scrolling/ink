import { ReadableStream, WritableStream, TransformStream, ByteLengthQueuingStrategy, CountQueuingStrategy } from "web-streams-polyfill";
import { Blob, File, FormData } from "./blob";
import { WebSocket, MessageEvent, CloseEvent } from "./websocket";
import { URL, URLSearchParams } from "whatwg-url";
import { fetch, Headers, Request, Response } from "./fetch";

Object.defineProperties(globalThis, {
  ...Object.fromEntries(Object.entries({ Blob, File, FormData, ReadableStream, WritableStream, TransformStream, ByteLengthQueuingStrategy, CountQueuingStrategy, WebSocket, MessageEvent, CloseEvent }).map(([name, value]) => [name, { value, writable: true, configurable: true }])),
  URL: { value: URL, writable: true, configurable: true },
  URLSearchParams: { value: URLSearchParams, writable: true, configurable: true },
  fetch: { value: fetch, writable: true, configurable: true },
  Headers: { value: Headers, writable: true, configurable: true },
  Request: { value: Request, writable: true, configurable: true },
  Response: { value: Response, writable: true, configurable: true },
});
