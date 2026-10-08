import { useEffect, useMemo, useRef, useState } from "react";
import { attachNativeController } from "ink/native/controller";
import { NativeError } from "ink/native";
import { playbackController } from "ink/internal/playback";

export interface AudioItem {
  id: string;
  src: string;
  title: string;
  artist?: string;
  album?: string;
  artwork?: string;
  duration?: number;
}
export type SourceReplacement = { prepare?: boolean } & ({ position?: number; offset?: never } | { offset: number; position?: never });

export interface PlayerState {
  ready: boolean;
  playing: boolean;
  playWhenReady: boolean;
  buffering: boolean;
  loading: boolean;
  ended: boolean;
  current: AudioItem | null;
  index: number;
  position: number;
  duration: number;
  speed: number;
  skipSilence: boolean;
  voiceBoost: boolean;
  silenceSaved: number;
  error: Error | null;
}
const initial: PlayerState = { ready: false, playing: false, playWhenReady: false, buffering: false, loading: false, ended: false, current: null, index: -1, position: 0, duration: 0, speed: 1, skipSilence: false, voiceBoost: false, silenceSaved: 0, error: null };

function decode(value: unknown): PlayerState {
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
  return {
    ready: field("ready") === true,
    playing: status === "playing",
    playWhenReady: field("playWhenReady") === true,
    buffering: status === "loading",
    loading: field("loading") === true,
    ended: status === "ended",
    index,
    current: src ? { id: text("id"), src, title: text("title"), artist: text("artist"), album: text("album"), artwork: text("artwork"), duration } : null,
    position: number("positionMs"), duration, speed: number("speed"),
    skipSilence: field("skipSilence") === true, voiceBoost: field("voiceBoost") === true,
    silenceSaved: number("silenceSavedMs"), error,
  };
}

export function usePlayer(options: { session?: string; mode?: "attached" | "detached"; usage?: "music" | "speech" } = {}) {
  const { session = "main", mode = "attached", usage = "music" } = options;
  if (!/^[A-Za-z0-9._-]{1,64}$/.test(session)) throw new TypeError("Audio session names must be 1–64 letters, numbers, dots, underscores or hyphens");
  const [state, setState] = useState(initial);
  const current = useRef(initial);
  const controller = useRef<ReturnType<typeof attachNativeController> | null>(null);
  useEffect(() => {
    const publish = (next: PlayerState) => { current.current = next; setState(next); };
    publish(initial);
    const attachment = attachNativeController("audio", { kind: "player", config: { session, playback: mode, usage } }, value => {
      try { publish(decode(value)); }
      catch (error) { publish({ ...initial, error: error instanceof Error ? error : new Error(String(error)) }); }
    });
    controller.current = attachment;
    void attachment.ready.catch(error => {
      if (controller.current === attachment) publish({ ...initial, error: error instanceof Error ? error : new Error(String(error)) });
    });
    return () => {
      controller.current = null;
      current.current = initial;
      void attachment.dispose().catch(error => console.error("Could not release audio player", error));
    };
  }, [session, mode, usage]);
  const commands = useMemo(() => {
    const call = async (operation: string, payload: unknown = {}) => {
      if (!controller.current || !current.current.ready) throw new NativeError("unavailable", "Audio player is not ready");
      await controller.current.call(operation, payload);
    };
    return {
      async getState(): Promise<PlayerState> {
        const active = controller.current;
        if (!active || !current.current.ready) throw new NativeError("unavailable", "Audio player is not ready");
        return decode(JSON.parse(await active.call("getState")));
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
      setQueue(items: readonly AudioItem[], options: { startIndex?: number; startPosition?: number; prepare?: boolean } = {}) {
        const startPosition = options.startPosition ?? 0;
        if (!Number.isFinite(startPosition) || startPosition < 0) return Promise.reject(new RangeError("Audio position must be a non-negative number of milliseconds"));
        return call("setQueue", { items, startIndex: options.startIndex ?? 0, startPosition, prepare: options.prepare ?? true });
      },
      play: () => call("play"),
      pause: () => call("pause"),
      toggle: () => call("toggle"),
      stop: () => call("stop"),
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
  }, []);
  return { state, [playbackController]: controller.current?.id, ...commands };
}
