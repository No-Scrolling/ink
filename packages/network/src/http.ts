import { ReadableStream } from "web-streams-polyfill";
import { callNative, callNativeBytes, NativeError } from "ink/native";

interface HttpRequest { url: string; method: string; headers: Readonly<Record<string, string>>; nativeParts?: Iterable<{ bytes: Uint8Array } | { src: string; offset: number; size: number }> }
interface HttpResponse { status: number; statusText: string; url: string; headers: [string, string][]; body: ReadableStream<Uint8Array> | null }

export async function requestHttp(request: HttpRequest, body: ReadableStream<Uint8Array> | null, signal: AbortSignal): Promise<HttpResponse> {
  let upload: string | undefined;
  if (body) {
    upload = await callNative("network", "stream-upload-open", { managed: request.nativeParts !== undefined }, { signal });
    const reader = request.nativeParts ? undefined : body.getReader();
    let rejectAbort: (reason: unknown) => void = () => {};
    const aborted = new Promise<never>((_, reject) => { rejectAbort = reject; });
    const abort = () => {
      if (reader) rejectAbort(signal.reason);
      void reader?.cancel(signal.reason).catch(() => {});
    };
    signal.addEventListener("abort", abort, { once: true });
    if (signal.aborted) abort();
    try {
      if (request.nativeParts) {
        for (const part of request.nativeParts) {
          if ("bytes" in part) await callNativeBytes("network", "stream-upload-write", { upload }, { signal, bytes: part.bytes });
          else await callNative("network", "stream-upload-file", { upload, src: part.src, offset: part.offset, size: part.size }, { signal });
        }
      } else if (reader) {
        for (;;) {
          const { value, done } = await Promise.race([reader.read(), aborted]);
          if (signal.aborted) throw signal.reason;
          if (done) break;
          if (!(value instanceof Uint8Array)) throw new TypeError("Upload streams must contain Uint8Array chunks");
          for (let offset = 0; offset < value.length; offset += 32 * 1024) {
            await callNativeBytes("network", "stream-upload-write", { upload }, { signal, bytes: value.subarray(offset, offset + 32 * 1024) });
          }
        }
      }
    } catch (error) {
      void reader?.cancel(error).catch(() => {});
      void callNative("network", "stream-upload-close", { upload }).catch(() => {});
      throw error;
    } finally {
      signal.removeEventListener("abort", abort);
      reader?.releaseLock();
    }
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
        const result = await callNativeBytes("network", "stream-read-bytes", { stream }, { signal });
        const chunk: unknown = JSON.parse(result.value);
        if (typeof chunk !== "object" || chunk === null || !("done" in chunk) || typeof chunk.done !== "boolean") throw new NativeError("protocol", "Invalid HTTP stream chunk");
        if (chunk.done) { await close(); controller.close(); }
        else controller.enqueue(result.bytes);
      } catch (error) { await close().catch(() => {}); controller.error(error); }
    },
    cancel: close,
  }, { highWaterMark: 0 });
  return { status: value.status, statusText: value.statusText, url: value.url, headers, body: responseBody };
}
