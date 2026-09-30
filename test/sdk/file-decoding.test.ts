import { expect, test } from "bun:test";
import { decodeFileRef } from "../../packages/files/src/decode";
import { NativeError } from "../../packages/ink/src/native";

const file = { id: "saved-1", src: "ink-file://saved-1", name: "Photo.jpg", mimeType: "image/jpeg", size: 1024 };

test("managed file decoding preserves optional metadata and the removed/cancelled null result", () => {
  const media = { ...file, width: 1920, height: 1080, duration: 1250.5 };
  expect(decodeFileRef(JSON.stringify(media))).toEqual(media);
  expect(decodeFileRef(JSON.stringify({ ...file, size: 0 }))).toEqual({ ...file, size: 0 });
  expect(decodeFileRef("null")).toBeNull();
});

test.each([
  { ...file, id: 4 }, { ...file, src: null }, { ...file, name: false }, { ...file, mimeType: [] },
  { ...file, size: "1024" }, { ...file, size: -1 }, { ...file, size: 0.5 },
  { ...file, size: Number.MAX_SAFE_INTEGER + 1 }, {}, [], "file", 7,
].map(value => ({ value })))("managed file decoding rejects malformed native fields: %j", ({ value }) => {
  try {
    decodeFileRef(JSON.stringify(value));
    throw Error("Expected a protocol failure");
  } catch (error) {
    expect(error).toBeInstanceOf(NativeError);
    expect((error as NativeError).kind).toBe("protocol");
  }
});

test.each(["width", "height", "duration"])("managed file decoding validates optional %s", field => {
  for (const value of [-1, "12", null, Infinity]) {
    expect(() => decodeFileRef(JSON.stringify({ ...file, [field]: value }))).toThrow(NativeError);
  }
  // JSON can encode non-finite magnitudes via an exponent even though
  // JSON.stringify converts JavaScript Infinity to null.
  const oversized = JSON.stringify({ ...file, [field]: "replace-me" }).replace('"replace-me"', "1e400");
  expect(() => decodeFileRef(oversized)).toThrow(NativeError);
});
