declare const __inkPost: (message: string) => void;

type Listener = (message: Record<string, unknown>) => void;
const listeners = new Map<string, Listener>();
const pending = new Map<number, {
  resolve: (value: string) => void;
  reject: (error: unknown) => void;
  dispose: () => void;
}>();
declare const __inkNextId: () => number;

export class NativeError extends Error {
  constructor(readonly kind: string, message: string, readonly retryable = false) {
    super(message);
    this.name = "NativeError";
  }
}

export function onNativeMessage(type: string, listener: Listener) {
  listeners.set(type, listener);
  return () => {
    if (listeners.get(type) === listener) listeners.delete(type);
  };
}

Object.defineProperty(globalThis, "__inkReceive", {
  value: (source: string) => {
    const message: unknown = JSON.parse(source);
    if (typeof message !== "object" || message === null || !("type" in message)
      || typeof message.type !== "string") throw new Error("Invalid native message");
    listeners.get(message.type)?.(message);
  },
});

onNativeMessage("result", message => {
  if (typeof message.id !== "number") throw new Error("Invalid native result ID");
  const request = pending.get(message.id);
  if (!request) return;
  pending.delete(message.id);
  request.dispose();
  if (typeof message.value === "string") request.resolve(message.value);
  else if (typeof message.kind === "string" && typeof message.message === "string") {
    request.reject(new NativeError(message.kind, message.message, message.retryable === true));
  } else request.reject(new NativeError("protocol", "Invalid native result"));
});

export function callNative(
  module: string,
  operation: string,
  payload: unknown,
  options: { signal?: AbortSignal; timeoutMs?: number; controller?: number } = {},
): Promise<string> {
  const { signal, timeoutMs = 30_000 } = options;
  if (options.controller !== undefined && (!Number.isSafeInteger(options.controller) || options.controller <= 0)) {
    return Promise.reject(new RangeError("Native controller ID must be a positive safe integer"));
  }
  if (signal?.aborted) return Promise.reject(signal.reason);
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0 || timeoutMs > 2_147_483_647) {
    return Promise.reject(new RangeError("Native request timeout must be a positive 32-bit integer"));
  }
  if (pending.size >= 256) return Promise.reject(new NativeError("busy", "Too many pending native requests", true));
  const id = __inkNextId();
  return new Promise((resolve, reject) => {
    const abort = () => {
      const request = pending.get(id);
      if (!request) return;
      pending.delete(id);
      request.dispose();
      reject(signal?.reason);
      __inkPost(JSON.stringify({ type: "cancel", id }));
    };
    pending.set(id, { resolve, reject, dispose: () => signal?.removeEventListener("abort", abort) });
    signal?.addEventListener("abort", abort, { once: true });
    try {
      __inkPost(JSON.stringify({ type: "call", id, module, operation, payload, timeoutMs, controller: options.controller }));
    } catch (error) {
      pending.delete(id);
      signal?.removeEventListener("abort", abort);
      const message = error instanceof Error ? error.message : String(error);
      reject(message.startsWith("busy:") ? new NativeError("busy", message.slice(6), true) : error);
    }
  });
}
