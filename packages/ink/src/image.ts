import { Icon, Stack, Text, type IconAsset } from "./index";
import { createElement, memo } from "./react";
import { navigate, type Destination } from "./navigation";

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
  return createElement("Row", {
    hasImage: image !== undefined,
    onPress: href === undefined ? onPress ?? (onLongPress ? () => {} : undefined) : () => navigate(href),
    onLongPress,
  },
    image !== undefined && createElement(Image, { src: image, width: 50, height: 50, fit: "cover" }),
    createElement(Stack, { gap: 0 },
      titleIcon === undefined ? createElement(Text, { size: 26, maxLines: titleMaxLines }, title)
        : createElement("RowTitle", { text: title, size: 26, maxLines: titleMaxLines },
          createElement(Icon, { name: titleIcon, size: 26 })),
      (subtitle !== undefined || subtitleIcon !== undefined) && createElement(Stack, { axis: "horizontal", align: "center", gap: 6 },
        subtitleIcon !== undefined && createElement(Icon, { name: subtitleIcon, size: 16 }),
        subtitle !== undefined && createElement(Text, { size: 16, maxLines: 1, tabularNumbers: true }, subtitle),
      ),
    ),
  );
});

export function Image(props: {
  src: string;
  width: number;
  height: number;
  fit?: "contain" | "cover";
  bleed?: boolean;
  zoomable?: boolean;
}) {
  return createElement("Image", props);
}
