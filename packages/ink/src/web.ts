export {};

// The bundler keeps this synchronous dependency uninitialised until first access.
declare function require(path: "./web-globals"): typeof import("./web-globals");

const names = [
  "Blob", "File", "FormData", "ReadableStream", "WritableStream", "TransformStream",
  "ByteLengthQueuingStrategy", "CountQueuingStrategy", "WebSocket", "MessageEvent",
  "CloseEvent", "URL", "URLSearchParams", "fetch", "Headers", "Request", "Response",
] as const;

for (const name of names) {
  Object.defineProperty(globalThis, name, {
    configurable: true,
    get() {
      const value = require("./web-globals")[name];
      Object.defineProperty(globalThis, name, { value, writable: true, configurable: true });
      return value;
    },
    set(value) {
      Object.defineProperty(globalThis, name, { value, writable: true, configurable: true });
    },
  });
}
