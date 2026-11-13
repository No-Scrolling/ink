import { NativeError } from "ink/native";
import type { AudioItem, PlayerState } from "./index";

function decodeItem(value: unknown): AudioItem {
  if (typeof value !== "object" || value === null) throw new NativeError("protocol", "Invalid audio queue item");
  const text = (name: string): string => {
    const result: unknown = Reflect.get(value, name);
    if (typeof result !== "string") throw new NativeError("protocol", `Invalid audio item ${name}`);
    return result;
  };
  const duration: unknown = Reflect.get(value, "duration");
  if (duration !== undefined && (typeof duration !== "number" || !Number.isFinite(duration) || duration < 0)) {
    throw new NativeError("protocol", "Invalid audio item duration");
  }
  return { id: text("id"), src: text("src"), title: text("title"), artist: text("artist"), album: text("album"), artwork: text("artwork"), duration, usage: decodeUsage(Reflect.get(value, "usage")) };
}

function decodeUsage(value: unknown): AudioItem["usage"] {
  if (value === undefined || value === "music" || value === "speech") return value;
  throw new NativeError("protocol", "Invalid audio item usage");
}

interface Queue { revision: number; items: readonly AudioItem[] }

export function decodePlayerState(value: unknown): PlayerState {
  return decodeState(value)[0];
}

export interface PlayerStateDecoder {
  (value: unknown): PlayerState;
  /** Records a queue this subscription is sending until its call settles. Native answers it with a revision and no items. */
  sending(items: readonly AudioItem[]): () => void;
}

/** Native sends the queue only when its revision changes, so each subscription keeps the last one it decoded. */
export function playerStateDecoder(): PlayerStateDecoder {
  let previous: Queue | undefined;
  const sent: (readonly AudioItem[])[] = [];
  const decode = (value: unknown) => {
    const [state, queue] = decodeState(value, previous, sent);
    previous = queue;
    return state;
  };
  return Object.assign(decode, {
    sending(items: readonly AudioItem[]) {
      // Native encodes missing text as empty and whole milliseconds, so the kept queue matches one it would send.
      const queue = items.map(({ id, src, title, artist = "", album = "", artwork = "", duration, usage }) =>
        ({ id, src, title, artist, album, artwork, duration: duration === undefined ? undefined : Math.trunc(duration), usage }));
      sent.push(queue);
      return () => { const index = sent.indexOf(queue); if (index >= 0) sent.splice(index, 1); };
    },
  });
}

function decodeState(value: unknown, previous?: Queue, sent: (readonly AudioItem[])[] = []): [PlayerState, Queue] {
  if (typeof value !== "object" || value === null) throw new NativeError("protocol", "Invalid player state");
  const field = (name: string): unknown => Reflect.get(value, name);
  const text = (name: string) => {
    const result = field(name);
    if (typeof result !== "string") throw new NativeError("protocol", `Invalid player ${name}`);
    return result;
  };
  const number = (name: string) => {
    const result = field(name);
    if (typeof result !== "number" || !Number.isFinite(result) || result < 0) throw new NativeError("protocol", `Invalid player ${name}`);
    return result;
  };
  const status = text("status");
  if (!["idle", "loading", "playing", "paused", "ended", "error"].includes(status) || typeof field("ready") !== "boolean") {
    throw new NativeError("protocol", "Invalid player status");
  }
  let error: Error | null = null;
  if (status === "error") {
    const detail = field("error");
    if (typeof detail !== "object" || detail === null
      || !("message" in detail) || typeof detail.message !== "string"
      || !("kind" in detail) || typeof detail.kind !== "string") throw new NativeError("protocol", "Invalid player error");
    error = new NativeError(detail.kind, detail.message, "retryable" in detail && detail.retryable === true);
  }
  const src = text("src");
  const index = field("index");
  if (typeof index !== "number" || !Number.isSafeInteger(index) || index < -1) {
    throw new NativeError("protocol", "Invalid player queue index");
  }
  const duration = number("durationMs");
  const revision = field("queueRevision");
  if (typeof revision !== "number" || !Number.isSafeInteger(revision)) throw new NativeError("protocol", "Invalid audio queue revision");
  const queue = field("queue");
  const repeat = field("repeat");
  if (repeat !== "off" && repeat !== "queue" && repeat !== "track") throw new NativeError("protocol", "Invalid repeat mode");
  const shuffle = field("shuffle");
  if (typeof shuffle !== "boolean") throw new NativeError("protocol", "Invalid shuffle mode");
  let items: readonly AudioItem[];
  if (Array.isArray(queue)) items = queue.map(decodeItem);
  else if (queue === undefined && previous?.revision === revision) items = previous.items;
  else if (queue === undefined && sent.length > 0) items = sent.shift()!;
  else throw new NativeError("protocol", "Invalid audio queue");
  return [{
    ready: field("ready") === true,
    playing: status === "playing",
    playWhenReady: field("playWhenReady") === true,
    buffering: status === "loading",
    loading: field("loading") === true,
    ended: status === "ended",
    queue: items,
    index,
    current: src ? { id: text("id"), src, title: text("title"), artist: text("artist"), album: text("album"), artwork: text("artwork"), duration, usage: decodeUsage(field("usage")) } : null,
    position: number("positionMs"), duration, speed: number("speed"),
    shuffle, repeat,
    skipSilence: field("skipSilence") === true, voiceBoost: field("voiceBoost") === true,
    silenceSaved: number("silenceSavedMs"), error,
  }, { revision, items }];
}
