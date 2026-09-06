import { fromByteArray, toByteArray } from "base64-js";
import { ReadableStream } from "web-streams-polyfill";
import { callNative, NativeError } from "./native";

interface HttpRequest { url: string; method: string; headers: Readonly<Record<string, string>>; nativeParts?: Iterable<{ bytes: string } | { src: string; offset: number; size: number }> }
interface HttpResponse { status: number; statusText: string; url: string; headers: [string, string][]; body: ReadableStream<Uint8Array> | null }

export async function requestHttp(request: HttpRequest, body: ReadableStream<Uint8Array> | null, signal: AbortSignal): Promise<HttpResponse> {
  let upload: string | undefined;
  if (body) {
    upload = await callNative("network", "stream-upload-open", { managed: request.nativeParts !== undefined }, { signal });
    const reader = request.nativeParts ? undefined : body.getReader();
    try {
      if (request.nativeParts) {
        for (const part of request.nativeParts) {
          if ("bytes" in part) await callNative("network", "stream-upload-write", { upload, bytes: part.bytes }, { signal });
          else for (let offset = 0; offset < part.size; offset += 32768) {
            await callNative("network", "stream-upload-file", { upload, src: part.src, offset: part.offset + offset, size: Math.min(32768, part.size - offset) }, { signal });
          }
        }
      } else if (reader) {
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          if (!(value instanceof Uint8Array)) throw new TypeError("Upload streams must contain Uint8Array chunks");
          for (let offset = 0; offset < value.length; offset += 32 * 1024) {
            await callNative("network", "stream-upload-write", { upload, bytes: fromByteArray(value.subarray(offset, offset + 32 * 1024)) }, { signal });
          }
        }
      }
    } catch (error) {
      await reader?.cancel(error).catch(() => {});
      await callNative("network", "stream-upload-close", { upload }).catch(() => {});
      throw error;
    } finally { reader?.releaseLock(); }
  }
  let value: unknown;
  try { value = JSON.parse(await callNative("network", "stream-open", { url: request.url, method: request.method, headers: request.headers, upload }, { signal })); }
  catch (error) {
    if (upload) await callNative("network", "stream-upload-close", { upload }).catch(() => {});
    throw error;
  }
  if (typeof value !== "object" || value === null
    || !("status" in value) || typeof value.status !== "number"
    || !("statusText" in value) || typeof value.statusText !== "string"
    || !("url" in value) || typeof value.url !== "string"
    || !("headers" in value) || !Array.isArray(value.headers)
    || !("stream" in value) || typeof value.stream !== "string") throw new NativeError("protocol", "Invalid HTTP response");
  const headers = value.headers.map((entry: unknown): [string, string] => {
    if (!Array.isArray(entry) || entry.length !== 2 || typeof entry[0] !== "string" || typeof entry[1] !== "string") throw new NativeError("protocol", "Invalid HTTP response header");
    return [entry[0], entry[1]];
  });
  const stream = value.stream;
  let closed = false;
  const close = async () => {
    if (closed) return;
    closed = true;
    signal.removeEventListener("abort", abort);
    await callNative("network", "stream-close", { stream });
  };
  let abort = () => {};
  const responseBody = new ReadableStream<Uint8Array>({
    start(controller) {
      abort = () => { controller.error(signal.reason); void close().catch(() => {}); };
      signal.addEventListener("abort", abort, { once: true });
      if (signal.aborted) abort();
    },
    async pull(controller) {
      try {
        const chunk: unknown = JSON.parse(await callNative("network", "stream-read", { stream }, { signal }));
        if (typeof chunk !== "object" || chunk === null || !("done" in chunk) || typeof chunk.done !== "boolean"
          || !("bytes" in chunk) || typeof chunk.bytes !== "string") throw new NativeError("protocol", "Invalid HTTP stream chunk");
        if (chunk.done) { await close(); controller.close(); }
        else controller.enqueue(toByteArray(chunk.bytes));
      } catch (error) { await close().catch(() => {}); controller.error(error); }
    },
    cancel: close,
  }, { highWaterMark: 0 });
  return { status: value.status, statusText: value.statusText, url: value.url, headers, body: responseBody };
}
