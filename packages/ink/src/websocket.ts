import { fromByteArray, toByteArray } from "base64-js";
import { URL } from "whatwg-url";
import { Blob } from "./blob";
import { callNative } from "./native";

export class MessageEvent extends Event {
  readonly data: unknown;
  readonly origin: string;
  readonly lastEventId: string;
  readonly source = null;
  readonly ports: readonly unknown[] = [];
  constructor(type: string, options: EventInit & { data?: unknown; origin?: string; lastEventId?: string } = {}) {
    super(type, options);
    this.data = options.data ?? null;
    this.origin = options.origin ?? "";
    this.lastEventId = options.lastEventId ?? "";
  }
}
export class CloseEvent extends Event {
  readonly code: number;
  readonly reason: string;
  readonly wasClean: boolean;
  constructor(type: string, options: EventInit & { code?: number; reason?: string; wasClean?: boolean } = {}) {
    super(type, options);
    this.code = options.code ?? 0;
    this.reason = options.reason ?? "";
    this.wasClean = options.wasClean ?? false;
  }
}

export class WebSocket extends EventTarget {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  readonly CONNECTING = 0;
  readonly OPEN = 1;
  readonly CLOSING = 2;
  readonly CLOSED = 3;
  readonly url: string;
  readonly extensions = "";
  protocol = "";
  binaryType: "blob" | "arraybuffer" = "blob";
  readyState = WebSocket.CONNECTING;
  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  #socket: Promise<string>;
  #outgoing = Promise.resolve();
  #queued = 0;
  #nativeQueued = 0;
  constructor(url: string | URL, protocols: string | string[] = []) {
    super();
    const parsed = new URL(String(url));
    if (parsed.protocol === "https:") parsed.protocol = "wss:";
    if ((parsed.protocol !== "wss:" && !(parsed.protocol === "ws:" && ["localhost", "127.0.0.1", "[::1]"].includes(parsed.hostname))) || parsed.hash || parsed.username || parsed.password) throw new DOMException("WebSocket requires a WSS URL without credentials or fragments", "SyntaxError");
    const names = typeof protocols === "string" ? [protocols] : protocols;
    if (new Set(names).size !== names.length || names.some(name => !/^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/.test(name))) throw new DOMException("Invalid WebSocket protocols", "SyntaxError");
    this.url = parsed.href;
    this.#socket = callNative("network", "socket-open", { url: this.url, protocols: names });
    void this.#receive();
  }
  get bufferedAmount() { return this.#queued + this.#nativeQueued; }
  #dispatch(type: string, properties: Record<string, unknown> = {}) {
    const event = type === "message" ? new MessageEvent(type, { data: properties.data, origin: String(properties.origin ?? "") })
      : type === "close" ? new CloseEvent(type, { code: Number(properties.code), reason: String(properties.reason), wasClean: properties.wasClean === true })
      : new Event(type);
    if (type !== "message" && type !== "close") Object.defineProperties(event, Object.fromEntries(Object.entries(properties).map(([key, value]) => [key, { value }])));
    this.dispatchEvent(event);
    try {
      if (type === "open") this.onopen?.(event);
      else if (type === "error") this.onerror?.(event);
      else if (event instanceof MessageEvent) this.onmessage?.(event);
      else if (event instanceof CloseEvent) this.onclose?.(event);
    } catch (error) { console.error(error); }
  }
  async #receive() {
    try {
      const socket = await this.#socket;
      while (this.readyState !== WebSocket.CLOSED) {
        const batch: { events: Record<string, unknown>[]; bufferedAmount: number } = JSON.parse(await callNative("network", "socket-read", { socket }));
        this.#nativeQueued = batch.bufferedAmount;
        for (const event of batch.events) {
          if (event.type === "open") {
            if (this.readyState !== WebSocket.CONNECTING) continue;
            this.readyState = WebSocket.OPEN;
            this.protocol = String(event.protocol);
            this.#dispatch("open");
          } else if (event.type === "message" && this.readyState === WebSocket.OPEN) {
            let data: string | Blob | ArrayBuffer;
            if (typeof event.text === "string") data = event.text;
            else {
              const bytes = toByteArray(String(event.bytes));
              data = this.binaryType === "arraybuffer" ? new Uint8Array(bytes).buffer : new Blob([bytes]);
            }
            this.#dispatch("message", { data, origin: new URL(this.url).origin, lastEventId: "", ports: [] });
          } else if (event.type === "error") this.#dispatch("error", { message: event.message });
          else if (event.type === "close") {
            this.readyState = WebSocket.CLOSED;
            this.#dispatch("close", { code: event.code, reason: event.reason, wasClean: event.wasClean });
          }
        }
      }
    } catch (error) {
      if (this.readyState !== WebSocket.CLOSED) {
        this.readyState = WebSocket.CLOSED;
        this.#dispatch("error", { error });
        this.#dispatch("close", { code: 1006, reason: "", wasClean: false });
      }
    } finally {
      await this.#socket.then(socket => callNative("network", "socket-dispose", { socket })).catch(() => {});
    }
  }
  send(data: string | ArrayBuffer | ArrayBufferView | Blob) {
    if (this.readyState === WebSocket.CONNECTING) throw new DOMException("WebSocket is connecting", "InvalidStateError");
    const blob = data instanceof Blob ? data : new Blob([data]);
    if (blob.size > 256 * 1024 || this.bufferedAmount + blob.size > 512 * 1024) throw new RangeError("WebSocket send queue exceeds its limit");
    this.#queued += blob.size;
    if (this.readyState !== WebSocket.OPEN) return;
    this.#outgoing = this.#outgoing.then(async () => {
      try {
        this.#nativeQueued = Number(await callNative("network", "socket-send", { socket: await this.#socket, text: typeof data === "string", bytes: fromByteArray(await blob.bytes()) }));
      } finally { this.#queued -= blob.size; }
    }).catch(error => { this.#dispatch("error", { error }); this.close(); });
  }
  close(code = 1000, reason = "") {
    if (code !== 1000 && (code < 3000 || code > 4999 || !Number.isInteger(code))) throw new DOMException("Invalid WebSocket close code", "InvalidAccessError");
    if (new TextEncoder().encode(reason).length > 123) throw new DOMException("WebSocket close reason exceeds 123 bytes", "SyntaxError");
    if (this.readyState >= WebSocket.CLOSING) return;
    this.readyState = WebSocket.CLOSING;
    void this.#outgoing.then(async () => callNative("network", "socket-close", { socket: await this.#socket, code, reason })).catch(() => {});
  }
}
