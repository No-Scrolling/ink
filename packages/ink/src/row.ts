import type { IconAsset } from "./assets";
import { createElement, memo } from "./react";
import type { Destination } from "./navigation";
import { pressHandler, type HitAreaProps, type PressProps } from "./press";

function RowComponent<const D extends Destination>({ image, title, titleMaxLines, titleIcon, subtitle, subtitleIcon, endIcon, endAction, href, url, onPress, onLongPress, hitSlop }: PressProps<D> & {
  image?: string;
  title: string;
  titleMaxLines?: number;
  titleIcon?: IconAsset;
  subtitle?: string;
  subtitleIcon?: IconAsset;
  /** A muted icon at the end of the row, such as an arrow for a link. Pressing it presses the row. */
  endIcon?: IconAsset;
  /** A separate button at the end of the row, such as one that removes the item. */
  endAction?: HitAreaProps & { icon: IconAsset; onPress: () => void };
  onLongPress?: () => void;
}) {
  return createElement("RowContent", {
    image, title, titleMaxLines, titleIcon, subtitle, subtitleIcon,
    endIcon: endAction?.icon ?? endIcon,
    onEndPress: endAction?.onPress,
    onPress: pressHandler({ href, url, onPress }) ?? (onLongPress ? () => {} : undefined),
    onLongPress,
    hitSlop,
    endHitSlop: endAction?.hitSlop,
  });
}

export const Row = /* @__PURE__ */ memo(RowComponent) as typeof RowComponent;

