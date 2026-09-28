import { createElement, useEffect, useRef, useState } from "react";
import { ErrorState } from "ink";
import { attachNativeController } from "ink/native/controller";

type Playback = { ready: boolean; rendered: boolean; position: number; duration: number };
const initial: Playback = { ready: false, rendered: false, position: 0, duration: 0 };

export function Video({ src }: { src: string }) {
  const [state, setState] = useState(initial);
  const [error, setError] = useState<Error | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [id, setId] = useState<number | null>(null);
  const controller = useRef<ReturnType<typeof attachNativeController> | null>(null);
  useEffect(() => {
    let active = true;
    setState(initial);
    setError(null);
    setId(null);
    const attachment = attachNativeController("video", { src }, value => {
      if (!active) return;
      if (typeof value !== "object" || value === null) return;
      if ("error" in value && typeof value.error === "string") setError(new Error(value.error));
      else if ("ready" in value && typeof value.ready === "boolean"
        && "rendered" in value && typeof value.rendered === "boolean"
        && "position" in value && typeof value.position === "number"
        && "duration" in value && typeof value.duration === "number") {
        setState({ ready: value.ready, rendered: value.rendered, position: value.position, duration: value.duration });
      }
    });
    controller.current = attachment;
    void attachment.ready.then(() => { if (active) setId(attachment.id); }, failure => { if (active) setError(failure); });
    return () => {
      active = false;
      controller.current = null;
      void attachment.dispose().catch(failure => console.error("Could not release video", failure));
    };
  }, [src, attempt]);
  function seek(position: number) {
    void controller.current?.call("seek", { position }).catch(setError);
  }
  if (error) return createElement(ErrorState, { message: error.message, onRetry: () => setAttempt(value => value + 1) });
  const loading = id === null || !state.ready || !state.rendered;
  return createElement("PlayingLayout", { hideControls: loading, bleed: true },
    createElement("VideoView", { controller: id, loading }),
    createElement("PlayingProgress", { clock: id, position: state.position, duration: state.duration,
      showTimes: true, onSeek: seek }),
  );
}
