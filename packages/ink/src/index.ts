/// <reference path="./assets.d.ts" />
import type { IconAsset } from "./assets";
import type { ReactNode } from "react";
import type {} from "react/jsx-runtime";
import type {} from "react/jsx-dev-runtime";
import { createElement } from "./react";
import { navigate, type Destination } from "./navigation";
export { useSnapshot, type Snapshot, type SnapshotSource } from "./snapshot";
export { resource, type ResourceSnapshot, type ResourceSource } from "./resource";
export { useAction, type Action } from "./action";
export { Tabs, Slot, navigate, replace, back, useRouteParams, type Destination } from "./navigation";

export function Screen({ header, children, leftAction, rightAction, ...props }: {
  children?: ReactNode;
  header?: ReactNode;
  title?: string;
  background?: string;
  centered?: boolean;
  wide?: boolean;
  bottomInset?: boolean;
  waitForImages?: boolean;
  leftAction?: { icon: IconAsset; onPress: () => void };
  rightAction?: { icon: IconAsset; onPress: () => void };
}) {
  return createElement("Screen", { ...props, pinnedHeader: header != null, leftIcon: leftAction?.icon, onLeftPress: leftAction?.onPress, rightIcon: rightAction?.icon, onRightPress: rightAction?.onPress },
    header != null && createElement(Stack, { gap: 47 }, header),
    children,
  );
}

export interface StackProps {
  children?: ReactNode;
  axis?: "vertical" | "horizontal";
  gap?: number;
  align?: "start" | "center" | "end" | "stretch";
  justify?: "start" | "center" | "end" | "space-between";
  onPress?: () => void;
  haptic?: boolean;
}

export const Stack = "Stack";

export interface TextProps {
  href?: string;
  onPress?: () => void;
  width?: number;
  children?: ReactNode;
  size?: number;
  align?: "start" | "center" | "end" | "justify";
  maxLines?: number;
  tabularNumbers?: boolean;
}

// A host element lets React reconcile text without an extra component wrapper.
export const Text = "Text";

declare module "react" {
  namespace JSX {
    interface IntrinsicElements {
      Text: TextProps;
      Stack: StackProps;
    }
  }
}

// The automatic JSX runtime can resolve a different React type instance in linked apps.
declare module "react/jsx-runtime" {
  namespace JSX {
    interface IntrinsicElements { Text: TextProps; Stack: StackProps; }
  }
}

declare module "react/jsx-dev-runtime" {
  namespace JSX {
    interface IntrinsicElements { Text: TextProps; Stack: StackProps; }
  }
}

export function Button(props: {
  children?: ReactNode;
  onPress?: () => void;
  onLongPress?: () => void;
  href?: Destination;
  disabled?: boolean;
  selected?: boolean;
  icon?: IconAsset;
}) {
  const { href, onPress, ...rest } = props;
  if (href !== undefined && onPress) throw new Error("Button accepts either href or onPress");
  return createElement("Button", { ...rest, onPress: href === undefined ? onPress : () => navigate(href) });
}

export { Row, Image, Avatar } from "./image";
export { Canvas, Rectangle, CanvasText, CanvasIcon } from "./canvas";

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
  subtitle?: string;
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

export { TextInput } from "./input";

export { List, type ListProps } from "./list";
export { ReorderList, type ReorderListProps } from "./reorder";
export { setColourScheme, useColourScheme, type ColourScheme } from "./appearance";
export {
  SettingsChoices, LoadingState, EmptyState, ErrorState, Confirmation,
  type SettingsChoice, type SettingsChoicesProps, type EmptyStateProps,
  type ErrorStateProps, type ConfirmationProps,
} from "./patterns";

export { findIcon, type IconAsset } from "./assets";
export { PlayingScreen, type PlayingScreenProps, type Playback } from "./playing";
export { ConversationScreen, Message, type ConversationScreenProps, type ConversationMessage, type MessageProps, type MessageAction, type ReplyPreview } from "./conversation";
export { openURL, share } from "./external";
export { LinkPreview, type LinkPreviewData, type LinkPreviewProps } from "./link-preview";
