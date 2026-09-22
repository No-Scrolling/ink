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
}) {
  return createElement("CanvasRectangle", props);
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
