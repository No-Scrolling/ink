import { Stack, Text } from "./index";
import { createElement } from "./react";
import { navigate, type Destination } from "./navigation";

export function Row({ image, title, subtitle, href, onPress }: {
  image?: string;
  title: string;
  subtitle?: string;
  href?: Destination;
  onPress?: () => void;
}) {
  if (href !== undefined && onPress) throw new Error("Row accepts either href or onPress");
  return createElement("Row", { hasImage: image !== undefined, onPress: href === undefined ? onPress : () => navigate(href) },
    image !== undefined && createElement(Image, { src: image, width: 50, height: 50, fit: "cover" }),
    createElement(Stack, { gap: 0 },
      createElement(Text, { size: 26 }, title),
      subtitle !== undefined && createElement(Text, { size: 16 }, subtitle),
    ),
  );
}

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

