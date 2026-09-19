import { createElement } from "react";
import { Image, Stack, Text } from "./index";
import { openLink } from "./external";

export type LinkPreviewData = {
  url: string;
  title?: string;
  image?: { src: string; width: number; height: number };
  icon?: string;
};

export type LinkPreviewProps = LinkPreviewData & {
  onPress?: () => void;
  onLongPress?: () => void;
};

export function LinkPreview({ url, title, image, icon, onPress, onLongPress }: LinkPreviewProps) {
  const domain = url.replace(/^https?:\/\/(?:www\.)?/, "").split(/[/?#]/, 1)[0];
  return createElement("Pressable", { onPress: onPress ?? (() => openLink(url)), onLongPress },
    createElement("LinkPreview", null,
      image && createElement(Image, {
        src: image.src, width: 220, height: 220 * image.height / image.width, fit: "cover",
      }),
      createElement(Stack, { gap: 4, align: "stretch" },
        title && createElement(Text, { size: 16, maxLines: 2 }, title),
        createElement(Stack, { axis: "horizontal", gap: 8, align: "center" },
          icon && createElement(Image, { src: icon, width: 16, height: 16, fit: "contain" }),
          createElement(Text, { size: 13, maxLines: 1 }, domain)))));
}
