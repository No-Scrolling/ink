import { expect, test } from "bun:test";
import { decodePlayerState } from "../../packages/audio/src/decode";
import { NativeError } from "../../packages/ink/src/native";

const item = { id: "episode-1", src: "https://example.com/episode.mp3", title: "Episode 1", artist: "", album: "", artwork: "" };
const state = {
  ...item, ready: true, status: "paused", index: 0, queue: [item],
  positionMs: 0, durationMs: 120_000, speed: 1, shuffle: false, repeat: "off", silenceSavedMs: 0,
};

test("audio state preserves content URIs in the current item and queue", () => {
  const src = "content://media/external/audio/media/42";
  const decoded = decodePlayerState({ ...state, src, queue: [{ ...item, src }] });
  expect(decoded.current?.src).toBe("content://media/external/audio/media/42");
  expect(decoded.queue[0]?.src).toBe("content://media/external/audio/media/42");
});

test("audio state preserves each item's usage and an omitted session fallback", () => {
  const decoded = decodePlayerState({ ...state, usage: "speech", queue: [
    { ...item, usage: "speech" }, { ...item, id: "music", usage: "music" }, { ...item, id: "fallback" },
  ] });
  expect(decoded.current?.usage).toBe("speech");
  expect(decoded.queue.map(item => item.usage)).toEqual(["speech", "music", undefined]);
  expect(decodePlayerState(state).current?.usage).toBeUndefined();
});

test.each(["alarm", "Speech", "", null, 2, false])("audio state rejects invalid item usage: %j", usage => {
  expect(() => decodePlayerState({ ...state, queue: [{ ...item, usage }] })).toThrow(NativeError);
  expect(() => decodePlayerState({ ...state, usage })).toThrow(NativeError);
});
