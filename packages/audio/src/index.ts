import { useEffect, useMemo, useRef, useState } from "ink/internal/react";
import { useNativeController } from "ink/internal/controller";
import { NativeError } from "ink/native";
import { playbackController } from "ink/internal/playback";
import { decodePlayerState } from "./decode";

export interface AudioItem {
  id: string;
  src: string;
  title: string;
  artist?: string;
  album?: string;
  artwork?: string;
  duration?: number;
  usage?: "music" | "speech";
}
export type SourceReplacement = { prepare?: boolean } & ({ position?: number; offset?: never } | { offset: number; position?: never });

export interface PlayerOptions {
  session?: string;
  mode?: "attached" | "detached";
  usage?: "music" | "speech";
  /** Reuse HTTP(S) audio in a shared 256 MiB disc cache. Off by default. */
  cache?: boolean;
  /** Milliseconds to buffer for the next queued item. Off by default. */
  preloadNext?: number;
  /** Milliseconds of audio to buffer before streaming starts. Defaults to 1000. */
  startupBuffer?: number;
}

export interface PlayerState {
  ready: boolean;
  playing: boolean;
  playWhenReady: boolean;
  buffering: boolean;
  loading: boolean;
  ended: boolean;
  queue: readonly AudioItem[];
  current: AudioItem | null;
  index: number;
  position: number;
  duration: number;
  speed: number;
  shuffle: boolean;
  repeat: "off" | "queue" | "track";
  skipSilence: boolean;
  voiceBoost: boolean;
  silenceSaved: number;
  error: Error | null;
}
const initial: PlayerState = { ready: false, playing: false, playWhenReady: false, buffering: false, loading: false, ended: false, queue: [], current: null, index: -1, position: 0, duration: 0, speed: 1, shuffle: false, repeat: "off", skipSilence: false, voiceBoost: false, silenceSaved: 0, error: null };

function validateItem(item: AudioItem) {
  validateId(item.id);
  if (item.usage !== undefined && item.usage !== "music" && item.usage !== "speech") throw new TypeError("Audio usage must be music or speech");
}

function validateId(id: string) {
  if (typeof id !== "string" || !id.trim()) throw new TypeError("Audio item IDs must not be blank");
}

function validateQueue(items: readonly AudioItem[]) {
  if (items.length === 0) throw new RangeError("Audio queues cannot be empty");
  const ids = new Set<string>();
  for (const item of items) {
    validateItem(item);
    if (ids.has(item.id)) throw new TypeError("Audio queue item IDs must be unique");
    ids.add(item.id);
  }
}

export function usePlayer(options: PlayerOptions = {}) {
  const { session = "main", mode = "attached", usage = "music", cache = false, preloadNext = 0, startupBuffer = 1000 } = options;
  if (!/^[A-Za-z0-9._-]{1,64}$/.test(session)) throw new TypeError("Audio session names must be 1–64 letters, numbers, dots, underscores or hyphens");
  if (usage !== "music" && usage !== "speech") throw new TypeError("Audio usage must be music or speech");
  if (!Number.isSafeInteger(preloadNext) || preloadNext < 0 || preloadNext > 30_000) throw new RangeError("Next-item preloading must be between 0 and 30000 milliseconds");
  if (!Number.isSafeInteger(startupBuffer) || startupBuffer < 0 || startupBuffer > 30_000) throw new RangeError("Startup buffering must be between 0 and 30000 milliseconds");
  const [state, setState] = useState(initial);
  const current = useRef(initial);
  const controller = useNativeController("audio", "audio player");
  useEffect(() => {
    const publish = (next: PlayerState) => { current.current = next; setState(next); };
    publish(initial);
    const detach = controller.attach({ kind: "player", config: { session, playback: mode, usage, cache, preloadNext, startupBuffer } }, value => {
      try { publish(decodePlayerState(value)); }
      catch (error) { publish({ ...initial, error: error instanceof Error ? error : new Error(String(error)) }); }
    }, error => {
      publish({ ...initial, error: error instanceof Error ? error : new Error(String(error)) });
    });
    return () => {
      detach();
      current.current = initial;
    };
  }, [session, mode, usage, cache, preloadNext, startupBuffer, controller]);
  const commands = useMemo(() => {
    const call = async (operation: string, payload: unknown = {}) => {
      if (!controller.current || !current.current.ready) throw new NativeError("unavailable", "Audio player is not ready");
      await controller.current.call(operation, payload);
    };
    return {
      async getState(): Promise<PlayerState> {
        const active = controller.current;
        if (!active || !current.current.ready) throw new NativeError("unavailable", "Audio player is not ready");
        return decodePlayerState(JSON.parse(await active.call("getState")));
      },
      seekBy(offset: number) {
        if (!Number.isSafeInteger(offset)) return Promise.reject(new RangeError("Seek offset must be a safe integer in milliseconds"));
        return call("seekBy", { value: offset });
      },
      replaceSource(id: string, src: string, options: SourceReplacement = {}) {
        if (options.position !== undefined && (!Number.isFinite(options.position) || options.position < 0)) {
          return Promise.reject(new RangeError("Audio position must be non-negative milliseconds"));
        }
        if (options.offset !== undefined && !Number.isSafeInteger(options.offset)) {
          return Promise.reject(new RangeError("Seek offset must be a safe integer in milliseconds"));
        }
        return call("replaceSource", { id, src, ...options });
      },
      async setQueue(items: readonly AudioItem[], options: { startIndex?: number; startPosition?: number; prepare?: boolean } = {}) {
        validateQueue(items);
        const startPosition = options.startPosition ?? 0;
        if (!Number.isFinite(startPosition) || startPosition < 0) throw new RangeError("Audio position must be a non-negative number of milliseconds");
        await call("setQueue", { items, startIndex: options.startIndex ?? 0, startPosition, prepare: options.prepare ?? true });
      },
      async add(item: AudioItem, options: { at?: number | "next" } = {}) {
        validateItem(item);
        if (options.at !== undefined && options.at !== "next" && (!Number.isSafeInteger(options.at) || options.at < 0)) {
          throw new RangeError("Audio insertion index must be a non-negative integer");
        }
        await call("add", { item, ...options });
      },
      remove: (id: string) => call("remove", { id }),
      async move(id: string, index: number) {
        if (!Number.isSafeInteger(index) || index < 0) throw new RangeError("Audio destination index must be a non-negative integer");
        await call("move", { id, index });
      },
      async replace(id: string, item: AudioItem) {
        validateItem(item);
        await call("replace", { id, item });
      },
      async play(id?: string) {
        if (id !== undefined) validateId(id);
        await call(id === undefined ? "play" : "playItem", { id });
      },
      prepare: () => call("prepare"),
      pause: () => call("pause"),
      toggle: () => call("toggle"),
      stop: () => call("stop"),
      setShuffle: (enabled: boolean) => call("setShuffle", { value: enabled }),
      setRepeat: (mode: PlayerState["repeat"]) => call("setRepeat", { value: mode }),
      setSpeed(speed: number) {
        if (!Number.isFinite(speed) || speed < 0.25 || speed > 4) return Promise.reject(new RangeError("Playback speed must be between 0.25 and 4"));
        return call("setSpeed", { value: speed });
      },
      setVoiceBoost: (enabled: boolean) => call("setVoiceBoost", { value: enabled }),
      setSkipSilence: (enabled: boolean) => call("setSkipSilence", { value: enabled }),
      next: () => call("next"),
      previous: () => call("previous"),
      seek(position: number) {
        if (!Number.isFinite(position) || position < 0) return Promise.reject(new RangeError("Audio position must be a non-negative number of milliseconds"));
        return call("seekTo", { value: position });
      },
    };
  }, [controller]);
  return { state, [playbackController]: controller.current?.id, ...commands };
}
