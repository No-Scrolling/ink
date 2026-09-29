import type { IconAsset } from "./index";
import { createElement, memo } from "./react";
import { navigate, type Destination } from "./navigation";

export function Avatar({ src = "", unread = false, onPress, onLongPress }: {
  src?: string;
  unread?: boolean;
  onPress?: () => void;
  onLongPress?: () => void;
}) {
  return createElement("Avatar", { src, unread, onPress, onLongPress });
}

export const Row = /* @__PURE__ */ memo(function Row({ image, title, titleMaxLines, titleIcon, subtitle, subtitleIcon, href, onPress, onLongPress }: {
  image?: string;
  title: string;
  titleMaxLines?: number;
  titleIcon?: IconAsset;
  subtitle?: string;
  subtitleIcon?: IconAsset;
  href?: Destination;
  onPress?: () => void;
  onLongPress?: () => void;
}) {
  if (href !== undefined && onPress) throw new Error("Row accepts either href or onPress");
  return createElement("RowContent", {
    image, title, titleMaxLines, titleIcon, subtitle, subtitleIcon,
    onPress: href === undefined ? onPress ?? (onLongPress ? () => {} : undefined) : () => navigate(href),
    onLongPress,
  });
});

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
