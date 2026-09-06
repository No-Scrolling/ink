/// <reference path="./assets.d.ts" />
import type { IconAsset } from "./assets";
import "./web";
import { createElement, useState, type ReactNode } from "react";
import { navigate, type Destination } from "./navigation";
export { useSnapshot, type Snapshot, type SnapshotSource } from "./snapshot";
export { useAction, type Action } from "./action";
export { Navigator, Route, Tabs, Tab, navigate, replace, back, useRouteParams, type Destination } from "./navigation";

export function Screen({ header, children, rightAction, ...props }: {
  children?: ReactNode;
  header?: ReactNode;
  title?: string;
  centered?: boolean;
  rightAction?: { icon: IconAsset; onPress: () => void };
}) {
  return createElement("Screen", { ...props, pinnedHeader: header != null, rightIcon: rightAction?.icon, onRightPress: rightAction?.onPress },
    header != null && createElement(Stack, { gap: 47 }, header),
    children,
  );
}

export function Stack(props: {
  children?: ReactNode;
  axis?: "vertical" | "horizontal";
  gap?: number;
  align?: "start" | "center" | "end" | "stretch";
  justify?: "start" | "center" | "end" | "space-between";
}) {
  return createElement("Stack", props);
}

export function Text(props: {
  children?: ReactNode;
  size?: number;
  align?: "start" | "center" | "end" | "justify";
  maxLines?: number;
}) {
  return createElement("Text", props);
}

export function Button(props: {
  children?: ReactNode;
  onPress?: () => void;
  href?: Destination;
  disabled?: boolean;
  selected?: boolean;
  icon?: IconAsset;
}) {
  const { href, onPress, ...rest } = props;
  if (href !== undefined && onPress) throw new Error("Button accepts either href or onPress");
  return createElement("Button", { ...rest, onPress: href === undefined ? onPress : () => navigate(href) });
}

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

export function Field(props: {
  label: string;
  children?: ReactNode;
  onPress?: () => void;
  href?: Destination;
}) {
  const { href, onPress, ...rest } = props;
  if (href !== undefined && onPress) throw new Error("Field accepts either href or onPress");
  return createElement("Field", { ...rest, onPress: href === undefined ? onPress : () => navigate(href) });
}

export function Toggle(props: {
  label: string;
  value: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
}) {
  return createElement("Toggle", props);
}

export function Icon(props: {
  name: IconAsset;
  size?: number;
  tone?: "primary" | "muted";
}) {
  return createElement("Icon", props);
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

export function TextInput(props: {
  value: string;
  onChange: (value: string) => void;
  onSubmit?: (value: string) => void;
  placeholder?: string;
  autoFocus?: boolean;
  action?: "return" | "search" | "done";
}) {
  const [eventCount, setEventCount] = useState(0);
  const nativeProps = {
    ...props,
    eventCount,
    onChange(value: string, count: number) {
      setEventCount(count);
      props.onChange(value);
    },
  };
  return createElement("TextInput", nativeProps);
}

export { List, type ListProps } from "./list";
export { setColourScheme, useColourScheme, type ColourScheme } from "./appearance";
export {
  SettingsChoices, LoadingState, EmptyState, ErrorState, Confirmation,
  type SettingsChoice, type SettingsChoicesProps, type EmptyStateProps,
  type ErrorStateProps, type ConfirmationProps,
} from "./patterns";

export { findIcon, type IconAsset } from "./assets";
export { PlayingScreen, type PlayingScreenProps } from "./playing";
export { ConversationScreen, Message, type ConversationScreenProps, type ConversationMessage, type MessageProps, type MessageAction, type ReplyPreview } from "./conversation";
export { openURL, share } from "./external";
