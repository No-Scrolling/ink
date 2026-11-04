import type { ReactNode } from "react";
import type { IconAsset } from "./assets";
import { createElement } from "./react";

type Bounds = { x: number; y: number; width: number; height: number };

export function Canvas(props: { width: number; height: number; children?: ReactNode }) {
  return createElement("Canvas", props);
}

export function Rectangle(props: Bounds & {
  fill?: string;
  stroke?: string;
  strokeWidth?: number;
  onPress?: () => void;
  onLongPress?: () => void;
  onDragEnter?: () => void;
  onDragStart?: () => void;
  onDragEnd?: () => void;
  haptic?: boolean;
}) {
  return createElement("CanvasRectangle", props);
}

/** Native-owned horizontal selection. Remount to reset its initial position. */
export function CanvasRange({
  total, span, initialStart, step = 1, snapPoints = [], snapDistance = 6, tapSnapDistance = 10, hitSlop = 10,
  minDragWidth = 0, inset = 4, innerFill = "#666666", dragOnly = false, snapOnTap = false, progress = 0,
  progressDurationMs = 0, progressPlaying = false, progressLoop = false, progressFill = "#FFFFFF", ...props
}: Bounds & {
  total: number;
  span: number;
  initialStart: number;
  step?: number;
  snapPoints?: readonly number[];
  /** Distances and inset use Canvas logical units. Drag release distance is 1.5× entry. */
  snapDistance?: number;
  tapSnapDistance?: number;
  hitSlop?: number;
  /** Minimum selection drag target width; does not change drawing or tap-to-seek. */
  minDragWidth?: number;
  inset?: number;
  fill?: string;
  innerFill?: string;
  /** Move only by dragging the selection; taps inside emit a relative seek. */
  dragOnly?: boolean;
  /** In drag-only mode, marker taps move the selection before considering a seek. */
  snapOnTap?: boolean;
  /** Playback fraction inside the selection, from zero to one. */
  progress?: number;
  /** Enables native playback interpolation; zero keeps progress static. */
  progressDurationMs?: number;
  /** Actual playback advancement, false while paused or buffering. */
  progressPlaying?: boolean;
  progressLoop?: boolean;
  progressFill?: string;
  onRangeSeek?: (position: number) => void;
  onRangeStart?: () => void;
  onRangeCommit?: (start: number) => void;
  /** FIFO snapshot before another press or Back; never emitted for movement. */
  onRangeSnapshot?: (start: number) => void;
}) {
  return createElement("CanvasRectangle", {
    ...props,
    range: {
      total, span, initialStart, step, snapPoints, snapDistance, tapSnapDistance, hitSlop, minDragWidth, inset,
      innerFill, dragOnly, snapOnTap, progress, progressDurationMs, progressPlaying, progressLoop, progressFill,
    },
  });
}

export function CanvasText({ children, ...props }: Bounds & {
  children: string | number;
  size?: number;
  colour?: string;
  align?: "start" | "center" | "end";
  tabularNumbers?: boolean;
}) {
  return createElement("CanvasText", { ...props, text: String(children) });
}

export function CanvasIcon({ size = 28, ...props }: {
  x: number;
  y: number;
  name: IconAsset;
  size?: number;
  colour?: string;
}) {
  return createElement("CanvasIcon", { ...props, width: size, height: size });
}
