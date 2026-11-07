import { createElement, useEffect, useState } from "react";
import { ErrorState } from "ink";
import { useNativeController } from "ink/internal/controller";

type Playback = { ready: boolean; playing: boolean; rendered: boolean; position: number; duration: number };
const initial: Playback = { ready: false, playing: false, rendered: false, position: 0, duration: 0 };

export function Video({ src }: { src: string }) {
  const [attempt, setAttempt] = useState(0);
  const video = usePlayback(src, true, attempt);
  if (video.error) return createElement(ErrorState, { message: video.error.message, onRetry: () => setAttempt(value => value + 1) });
  const loading = video.id === null || !video.ready || !video.rendered;
  return createElement("PlayingLayout", { hideControls: loading, bleed: true },
    createElement("VideoView", { controller: video.id, loading }),
    createElement(VideoProgress, { video }),
  );
}

function usePlayback(src: string | null, controls: boolean, attempt = 0) {
  const [state, setState] = useState(initial);
  const [error, setError] = useState<Error | null>(null);
  const [id, setId] = useState<number | null>(null);
  const controller = useNativeController("video");
  useEffect(() => {
    setState(initial);
    setError(null);
    setId(null);
    if (!src) return;
    return controller.attach({ src, controls }, value => {
      if (typeof value !== "object" || value === null) return;
      if ("error" in value && typeof value.error === "string") setError(new Error(value.error));
      else if ("ready" in value && typeof value.ready === "boolean"
        && "playing" in value && typeof value.playing === "boolean"
        && "rendered" in value && typeof value.rendered === "boolean"
        && "position" in value && typeof value.position === "number"
        && "duration" in value && typeof value.duration === "number") {
        setState({ ready: value.ready, playing: value.playing, rendered: value.rendered, position: value.position, duration: value.duration });
      }
    }, setError, setId);
  }, [src, controls, attempt, controller]);
  function seek(position: number, exact = true) {
    void controller.current?.call("seek", { position, exact }).catch(setError);
  }
  const toggle = () => { void controller.current?.call(state.playing ? "pause" : "play").catch(setError); };
  return { id, ...state, error, seek, toggle };
}

export type InlineVideo = {
  /** Pass to `Image`'s `videoController` to draw the video over the image. */
  id: number | null;
  playing: boolean;
  /** Whether a frame is showing; until then the poster is visible. */
  rendered: boolean;
  /** Seconds. */
  position: number;
  duration: number;
  /** Seeks to seconds; quick seeks may land on a nearby keyframe. */
  seek: (position: number, exact?: boolean) => void;
  error: Error | null;
  toggle: () => void;
};

/**
 * A paused video without controls for drawing over an image. Touches reach the app, so the
 * image can still be swiped; call `toggle` from its `onPress`.
 */
export function useVideo(src: string | null): InlineVideo {
  return usePlayback(src, false);
}

/** Seekable progress for a video from `useVideo`, updated natively while it plays. */
export function VideoProgress({ video }: { video: InlineVideo }) {
  return createElement("PlayingProgress", { clock: video.id, position: video.position, duration: video.duration,
    showTimes: true, onSeek: video.seek });
}
