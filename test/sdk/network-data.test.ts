import { expect, test } from "bun:test";
import { resolve } from "node:path";
const { ReadableStream } = await import(Bun.resolveSync("web-streams-polyfill", resolve(import.meta.dir, "../../packages/network")));
const { URLSearchParams } = await import(Bun.resolveSync("whatwg-url", resolve(import.meta.dir, "../../packages/network")));
import { Headers, Request, Response } from "../../packages/network/src/fetch";
import { Blob, File, FormData, multipart, parseFormData, nativeBlob, nativeBlobParts } from "../../packages/network/src/blob";


test("Headers normalises names and whitespace, combines ordinary fields and preserves separate cookies", () => {
  const headers = new Headers([["X-Trace", "  first\t"], ["x-trace", "second"], ["Set-Cookie", "a=1"], ["set-cookie", "b=2"]]);
  expect(headers.get("X-TRACE")).toBe("first, second");
  expect(headers.getSetCookie()).toEqual(["a=1", "b=2"]);
  expect([...headers]).toEqual([["set-cookie", "a=1"], ["set-cookie", "b=2"], ["x-trace", "first, second"]]);
  const copied = new Headers(headers);
  copied.set("X-Trace", "replacement");
  copied.delete("set-cookie");
  expect(headers.get("x-trace")).toBe("first, second");
  expect(headers.getSetCookie()).toEqual(["a=1", "b=2"]);
  expect(copied.get("X-TRACE")).toBe("replacement");
  expect(copied.has("set-cookie")).toBe(false);
});

test.each(["bad name", "", "name:other", "na\rme"])("Headers rejects invalid field names: %j", name => {
  expect(() => new Headers({ [name]: "value" })).toThrow(TypeError);
});

test.each(["x\r\ny", "x\ny", "x\u0000y", "\u0100"])("Headers rejects invalid field values: %j", value => {
  expect(() => new Headers({ test: value })).toThrow(TypeError);
});

test("Request normalises URL and method and infers the encoded body type", async () => {
  const request = new Request("https://example.test/a/../upload?q=1", {
    method: "post", body: new URLSearchParams([["name", "Zoë"], ["tag", "a b"]]),
  });
  expect(request.url).toBe("https://example.test/upload?q=1");
  expect(request.method).toBe("POST");
  expect(request.headers.get("content-type")).toBe("application/x-www-form-urlencoded;charset=UTF-8");
  expect(await request.text()).toBe("name=Zo%C3%AB&tag=a+b");
  expect(request.bodyUsed).toBe(true);
  await expect(request.text()).rejects.toThrow(TypeError);
});

test("Request cloning leaves independently readable bodies and headers", async () => {
  const original = new Request("https://example.test/", { method: "POST", body: "payload", headers: { "x-test": "one" } });
  const copy = original.clone();
  expect(original.bodyUsed).toBe(false);
  expect(copy.bodyUsed).toBe(false);
  copy.headers.set("x-test", "two");
  expect(original.headers.get("x-test")).toBe("one");
  expect(await copy.text()).toBe("payload");
  expect(await original.text()).toBe("payload");
  expect(() => original.clone()).toThrow(TypeError);
});

test("constructing from Request transfers ownership and locked bodies cannot be cloned", async () => {
  const original = new Request("https://example.test/", { method: "POST", body: "once" });
  const transferred = new Request(original);
  expect(original.bodyUsed).toBe(true);
  await expect(original.text()).rejects.toThrow(TypeError);
  expect(await transferred.text()).toBe("once");
  const locked = new Request("https://example.test/", { method: "POST", body: "locked" });
  const reader = locked.body!.getReader();
  expect(() => locked.clone()).toThrow(TypeError);
  reader.releaseLock();
  expect(await locked.text()).toBe("locked");
});

test.each([
  { url: "https://user:secret@example.test/", init: {}, error: TypeError },
  { url: "https://example.test/", init: { method: "TRACE" }, error: TypeError },
  { url: "https://example.test/", init: { method: "bad method" }, error: TypeError },
  { url: "https://example.test/", init: { method: "HEAD", body: "data" }, error: TypeError },
  { url: "https://example.test/", init: { method: "GET", body: "" }, error: TypeError },
])("Request rejects unsafe or unsupported construction: $url $init", ({ url, init, error }) => {
  expect(() => new Request(url, init)).toThrow(error);
});

test("Response clone preserves metadata, does not consume either branch and JSON validates serialisability", async () => {
  const original = Response.json({ count: 3, label: "Málaga" }, { status: 201, headers: { "x-source": "local" } });
  const copy = original.clone();
  expect(copy.status).toBe(201);
  expect(copy.ok).toBe(true);
  expect(copy.headers.get("content-type")).toBe("application/json");
  expect(copy.headers.get("x-source")).toBe("local");
  expect(original.bodyUsed).toBe(false);
  expect(await original.json()).toEqual({ count: 3, label: "Málaga" });
  expect(await copy.text()).toBe('{"count":3,"label":"Málaga"}');
  expect(() => Response.json(undefined)).toThrow(TypeError);
  expect(() => Response.json(1n)).toThrow(TypeError);
});

test("empty responses remain reusable and forbidden status/body combinations fail", async () => {
  const empty = new Response(null, { status: 204 });
  expect(await empty.text()).toBe("");
  expect(await empty.bytes()).toEqual(new Uint8Array());
  expect(empty.bodyUsed).toBe(false);
  for (const status of [204, 205, 304]) expect(() => new Response("", { status })).toThrow(TypeError);
  for (const status of [199, 600, 200.5, NaN]) expect(() => new Response(null, { status })).toThrow(RangeError);
  expect(() => new Response(null, { statusText: "OK\r\nInjected: yes" })).toThrow(TypeError);
});

test("redirect response headers remain immutable after cloning", () => {
  const original = Response.redirect("https://example.test/a/../next", 307);
  expect(original.status).toBe(307);
  expect(original.headers.get("location")).toBe("https://example.test/next");
  for (const response of [original, original.clone()]) {
    expect(() => response.headers.set("location", "https://other.test/")).toThrow(TypeError);
    expect(() => response.headers.append("x-test", "yes")).toThrow(TypeError);
    expect(() => response.headers.delete("location")).toThrow(TypeError);
  }
  expect(() => Response.redirect("https://example.test/", 200)).toThrow(RangeError);
});

test("buffering limit cancels an oversized stream and makes its body unavailable", async () => {
  const reason = { aborted: true };
  let cancelled: unknown;
  const stream = new ReadableStream<Uint8Array>({
    pull(controller) { controller.enqueue(new Uint8Array(16 * 1024 * 1024 + 1)); },
    cancel(value) { cancelled = value; },
  }, { highWaterMark: 0 });
  const response = new Response(stream);
  await expect(response.bytes()).rejects.toThrow(RangeError);
  expect(cancelled).toBeInstanceOf(RangeError);
  expect(response.bodyUsed).toBe(true);
  await expect(response.text()).rejects.toThrow(TypeError);
  const controller = new AbortController();
  controller.abort(reason);
  await expect(new Response("data", { signal: controller.signal }).text()).rejects.toBe(reason);
});

test("Blob snapshots typed-array views and slices across UTF-8 and binary part boundaries", async () => {
  const buffer = new Uint8Array([0, 10, 20, 30, 255]);
  const blob = new Blob(["é", buffer.subarray(1, 4), new Blob(["Z"])], { type: "APPLICATION/OCTET-STREAM" });
  buffer.fill(99);
  expect(blob.size).toBe(6);
  expect(blob.type).toBe("application/octet-stream");
  expect(await blob.bytes()).toEqual(new Uint8Array([195, 169, 10, 20, 30, 90]));
  expect(await blob.slice(1, -1).bytes()).toEqual(new Uint8Array([169, 10, 20, 30]));
  expect(blob.slice(5, 2).size).toBe(0);
  expect(await blob.slice(5, 2).bytes()).toEqual(new Uint8Array());
  expect(blob.slice(0, 2, "TEXT/PLAIN").type).toBe("text/plain");
});

test("managed Blob slicing retains file ranges without eagerly reading the file", () => {
  const blob = new Blob(["AB", nativeBlob("ink-file://file-7", 100, "image/jpeg"), "XY"]);
  expect(blob.size).toBe(104);
  expect([...nativeBlobParts(blob.slice(1, 103))!]).toEqual([
    { bytes: new Uint8Array([66]) }, { src: "ink-file://file-7", offset: 0, size: 100 }, { bytes: new Uint8Array([88]) },
  ]);
  expect([...nativeBlobParts(blob.slice(12, 22))!]).toEqual([{ src: "ink-file://file-7", offset: 10, size: 10 }]);
});

test("FormData set preserves the first field position and removes duplicate names", async () => {
  const form = new FormData();
  form.append("first", "one");
  form.append("tag", "old");
  form.append("last", "end");
  form.append("tag", "duplicate");
  form.set("tag", "new");
  expect([...form]).toEqual([["first", "one"], ["tag", "new"], ["last", "end"]]);
  form.append("upload", new Blob(["file"], { type: "TEXT/PLAIN" }), "note.txt");
  const file = form.get("upload") as File;
  expect(file.name).toBe("note.txt");
  expect(file.type).toBe("text/plain");
  expect(await file.text()).toBe("file");
  form.delete("tag");
  expect(form.getAll("tag")).toEqual([]);
});

test("multipart emission frames binary files and escaping without altering file bytes", async () => {
  const form = new FormData();
  form.append('a"\nb', "one\ntwo");
  form.append("file", new Blob([new Uint8Array([0, 255, 13, 10])], { type: "application/octet-stream" }), "data.bin");
  const blob = multipart(form);
  const boundary = blob.type.split("boundary=")[1];
  const prefix = new TextEncoder().encode(`--${boundary}\r\nContent-Disposition: form-data; name="a%22%0D%0Ab"\r\n\r\none\r\ntwo\r\n--${boundary}\r\nContent-Disposition: form-data; name="file"; filename="data.bin"\r\nContent-Type: application/octet-stream\r\n\r\n`);
  const suffix = new TextEncoder().encode(`\r\n--${boundary}--\r\n`);
  const expected = new Uint8Array(prefix.length + 4 + suffix.length);
  expected.set(prefix);
  expected.set([0, 255, 13, 10], prefix.length);
  expected.set(suffix, prefix.length + 4);
  expect(await blob.bytes()).toEqual(expected);
});

test("form decoding accepts a fixed multipart wire fixture and repeated URL-encoded fields", async () => {
  const bytes = new TextEncoder().encode('--fixture\r\nContent-Disposition: form-data; name="label"\r\n\r\nhello\r\n--fixture\r\nContent-Disposition: form-data; name="file"; filename="note.txt"\r\nContent-Type: text/plain\r\n\r\nfile data\r\n--fixture--\r\n');
  const form = parseFormData(bytes, 'multipart/form-data; boundary="fixture"');
  expect(form.get("label")).toBe("hello");
  const file = form.get("file") as File;
  expect(file.name).toBe("note.txt");
  expect(file.type).toBe("text/plain");
  expect(await file.text()).toBe("file data");
  const encoded = parseFormData(new TextEncoder().encode("tag=one&tag=two&label=a+b%26c"), "application/x-www-form-urlencoded;charset=UTF-8");
  expect([...encoded]).toEqual([["tag", "one"], ["tag", "two"], ["label", "a b&c"]]);
});

test("multipart parser accepts permitted whitespace after boundary delimiters", async () => {
  // RFC 2046 section 5.1.1 permits linear whitespace between the boundary
  // value and its terminating CRLF. These independently authored wire bytes
  // exercise the parser rather than trusting its companion encoder.
  const bytes = new TextEncoder().encode('--fixture\r\nContent-Disposition: form-data; name="label"\r\n\r\nhello\r\n--fixture \t\r\nContent-Disposition: form-data; name="file"; filename="note.txt"\r\nContent-Type: text/plain\r\n\r\nfile data\r\n--fixture--\r\n');
  const form = parseFormData(bytes, "multipart/form-data; boundary=fixture");
  expect(form.get("label")).toBe("hello");
  expect(await (form.get("file") as File).text()).toBe("file data");
});

test.each([" ", "\t"])("multipart opening boundary accepts horizontal padding %j", padding => {
  const bytes = new TextEncoder().encode(`--fixture${padding}\r\nContent-Disposition: form-data; name="label"\r\n\r\nhello\r\n--fixture--\r\n`);
  expect(parseFormData(bytes, "multipart/form-data; boundary=fixture").get("label")).toBe("hello");
});

test.each(["\v", "\n"])("multipart opening boundary rejects non-horizontal padding %j", padding => {
  const bytes = new TextEncoder().encode(`--fixture${padding}\r\nContent-Disposition: form-data; name="label"\r\n\r\nhello\r\n--fixture--\r\n`);
  expect(() => parseFormData(bytes, "multipart/form-data; boundary=fixture")).toThrow("Invalid multipart framing");
});
