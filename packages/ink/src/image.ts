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

export function Image(props: {
  src: string;
  width: number;
  height: number;
  fit?: "contain" | "cover";
  bleed?: boolean;
  /** Fill the available content width, using width and height as the aspect ratio. */
  fillWidth?: boolean;
  zoomable?: boolean;
  loop?: boolean;
}) {
  return createElement("Image", props);
}
