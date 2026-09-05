(() => {
  globalThis.TextEncoder = class TextEncoder {
    get encoding() { return "utf-8"; }
    encode(input = "") {
      return __inkEncodeUtf8(String(input).replace(/[\uD800-\uDFFF]/gu, "\uFFFD"));
    }
    encodeInto(input, destination) {
      if (!(destination instanceof Uint8Array)) throw new TypeError("Expected Uint8Array");
      const source = String(input);
      const bytes = this.encode(source);
      let written = Math.min(bytes.length, destination.length);
      if (written < bytes.length) {
        while (written > 0 && (bytes[written] & 0xC0) === 0x80) written--;
      }
      destination.set(bytes.subarray(0, written));
      let read = 0;
      for (let offset = 0; offset < written;) {
        const byte = bytes[offset];
        offset += byte < 0x80 ? 1 : byte < 0xE0 ? 2 : byte < 0xF0 ? 3 : 4;
        read += byte >= 0xF0 ? 2 : 1;
      }
      return { read, written };
    }
  };

  globalThis.TextDecoder = class TextDecoder {
    #pending = new Uint8Array();
    #started = false;
    #fatal;
    #ignoreBOM;
    constructor(label = "utf-8", options = {}) {
      if (!["utf-8", "utf8", "unicode-1-1-utf-8"].includes(String(label).trim().toLowerCase())) {
        throw new RangeError("Only UTF-8 decoding is supported");
      }
      this.#fatal = Boolean(options.fatal);
      this.#ignoreBOM = Boolean(options.ignoreBOM);
    }
    get encoding() { return "utf-8"; }
    get fatal() { return this.#fatal; }
    get ignoreBOM() { return this.#ignoreBOM; }
    decode(input = new Uint8Array(), options = {}) {
      let bytes;
      if (input instanceof ArrayBuffer) bytes = new Uint8Array(input);
      else if (ArrayBuffer.isView(input)) bytes = new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
      else throw new TypeError("Expected an ArrayBuffer or view");
      if (this.#pending.length) {
        const combined = new Uint8Array(this.#pending.length + bytes.length);
        combined.set(this.#pending);
        combined.set(bytes, this.#pending.length);
        bytes = combined;
      }
      const stream = Boolean(options.stream);
      let text;
      try {
        const [decoded, consumed] = __inkDecodeUtf8(bytes, this.#fatal, stream);
        text = decoded;
        this.#pending = bytes.slice(consumed);
      } catch (error) {
        this.#pending = new Uint8Array();
        this.#started = false;
        throw error;
      }
      if (text.length && !this.#started) {
        this.#started = true;
        if (!this.#ignoreBOM && text.charCodeAt(0) === 0xFEFF) text = text.slice(1);
      }
      if (!stream) this.#started = false;
      return text;
    }
  };
})();
