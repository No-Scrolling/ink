import { expect, test } from "bun:test";
import { decodePlayerState, playerStateDecoder } from "../../packages/audio/src/decode";
import { NativeError } from "../../packages/ink/src/native";

const item = { id: "episode-1", src: "https://example.com/episode.mp3", title: "Episode 1", artist: "", album: "", artwork: "" };
const state = {
  ...item, ready: true, status: "paused", index: 0, queue: [item], queueRevision: 0,
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

test("audio state keeps the same queue until native sends a new revision", () => {
  const decode = playerStateDecoder();
  const first = decode(state);
  const playing = decode({ ...state, status: "playing", positionMs: 5_000, queue: undefined });
  expect(playing.playing).toBe(true);
  expect(playing.position).toBe(5_000);
  expect(playing.queue).toBe(first.queue);
  const next = { ...item, id: "episode-2", title: "Episode 2" };
  const edited = decode({ ...state, queueRevision: 1, queue: [item, next] });
  expect(edited.queue.map(item => item.id)).toEqual(["episode-1", "episode-2"]);
  expect(decode({ ...state, queueRevision: 1, queue: undefined }).queue).toBe(edited.queue);
});

test("audio state rejects a missing queue after a revision change", () => {
  const decode = playerStateDecoder();
  expect(() => decode({ ...state, queue: undefined })).toThrow(NativeError);
  decode(state);
  expect(() => decode({ ...state, queueRevision: 1, queue: undefined })).toThrow(NativeError);
  expect(() => decodePlayerState({ ...state, queue: undefined })).toThrow(NativeError);
});

test("audio state keeps a queue the app sent when native answers with its revision only", () => {
  const decode = playerStateDecoder();
  decode(state);
  const sent = [{ id: "a", src: "https://example.com/a.mp3", title: "A", duration: 1_500.7 }, { id: "b", src: "https://example.com/b.mp3", title: "B", artist: "Artist", usage: "speech" as const }];
  const settled = decode.sending(sent);
  const answered = decode({ ...state, queueRevision: 4, index: 1, queue: undefined });
  expect(answered.queue).toEqual([
    { id: "a", src: "https://example.com/a.mp3", title: "A", artist: "", album: "", artwork: "", duration: 1_500, usage: undefined },
    { id: "b", src: "https://example.com/b.mp3", title: "B", artist: "Artist", album: "", artwork: "", duration: undefined, usage: "speech" },
  ]);
  expect(answered.index).toBe(1);
  settled();
  expect(decode({ ...state, queueRevision: 4, queue: undefined }).queue).toBe(answered.queue);
  // A call that settles without its revision-only answer leaves nothing for a later revision.
  decode.sending(sent)();
  expect(() => decode({ ...state, queueRevision: 5, queue: undefined })).toThrow(NativeError);
});
