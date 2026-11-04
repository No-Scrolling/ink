import type { IconAsset } from "./index";
import { createElement, memo } from "./react";
import type { Destination } from "./navigation";
import { pressHandler, type PressProps } from "./press";

export function Avatar({ src = "", unread = false, onPress, onLongPress }: {
  src?: string;
  unread?: boolean;
  onPress?: () => void;
  onLongPress?: () => void;
}) {
  return createElement("Avatar", { src, unread, onPress, onLongPress });
}

function RowComponent<const D extends Destination>({ image, title, titleMaxLines, titleIcon, subtitle, subtitleIcon, href, url, onPress, onLongPress }: PressProps<D> & {
  image?: string;
  title: string;
  titleMaxLines?: number;
  titleIcon?: IconAsset;
  subtitle?: string;
  subtitleIcon?: IconAsset;
  onLongPress?: () => void;
}) {
  return createElement("RowContent", {
    image, title, titleMaxLines, titleIcon, subtitle, subtitleIcon,
    onPress: pressHandler({ href, url, onPress }) ?? (onLongPress ? () => {} : undefined),
    onLongPress,
  });
}

export const Row = /* @__PURE__ */ memo(RowComponent) as typeof RowComponent;

export type ImageCrop = { zoom: number; x: number; y: number };
export type ImageOverlay = { src: string; x: number; y: number; width: number; height: number };

export function Image(props: {
  src: string;
  width: number;
  height: number;
  fit?: "contain" | "cover";
  /** Clockwise quarter turns. Reuses one decoded texture across rotation and fit changes. */
  rotation?: 0 | 90 | 180 | 270;
  background?: string;
  /** Normalised centre in the rotated source, with zoom relative to cover (1–4). */
  crop?: ImageCrop;
  /** Native gesture updates, delivered before subsequent press callbacks. */
  onCropChange?: (crop: ImageCrop) => void;
  /** Dashed horizontal guides at normalised frame positions (0–1), independent of crop. */
  horizontalGuides?: readonly number[];
  /** Transparent image in normalised frame coordinates, independent of crop and rotation. */
  overlay?: ImageOverlay;
  bleed?: boolean;
  /** Fill the available content width, using width and height as the aspect ratio. */
  fillWidth?: boolean;
  zoomable?: boolean;
  loop?: boolean;
}) {
  return createElement("Image", props);
}
