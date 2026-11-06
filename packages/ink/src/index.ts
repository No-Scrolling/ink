/// <reference path="./assets.d.ts" />
import type { IconAsset } from "./assets";
import type { ReactElement, ReactNode } from "react";
import type {} from "react/jsx-runtime";
import type {} from "react/jsx-dev-runtime";
import { createElement, useMemo, useState } from "./react";
import { ScrollToEndContext } from "./scroll";
import type { Destination } from "./navigation";
import { pressHandler, type PressProps } from "./press";
export { useSnapshot, type Snapshot, type SnapshotSource } from "./snapshot";
export { resource, type ResourceSnapshot, type ResourceSource } from "./resource";
export { useAction, type Action } from "./action";
export { NativeError } from "./native";
export { Tabs, Slot, navigate, replace, back, useRouteParams, type Destination } from "./navigation";

export function Screen({ header, children, leftAction, rightAction, ...props }: {
  children?: ReactNode;
  header?: ReactNode;
  title?: string;
  /** Shown under the title when the theme uses the large header layout. */
  subtitle?: string;
  /** Small muted text pinned to the bottom of the screen, such as a hint or privacy note. */
  note?: string;
  /** A named variant from `variants` in theme.ts. The build fails if theme.ts does not define it. */
  theme?: string;
  background?: string;
  centered?: boolean;
  wide?: boolean;
  bottomInset?: boolean;
  leftAction?: { icon: IconAsset; onPress: () => void };
  rightAction?: { icon: IconAsset; onPress: () => void };
}) {
  const [scrollToEnd, setScrollToEnd] = useState(0);
  const requestEnd = useMemo(() => () => setScrollToEnd(value => value + 1), []);
  return createElement(ScrollToEndContext.Provider, { value: requestEnd }, createElement("Screen", { ...props, scrollToEnd, pinnedHeader: header != null, leftIcon: leftAction?.icon, onLeftPress: leftAction?.onPress, rightIcon: rightAction?.icon, onRightPress: rightAction?.onPress },
    header != null && createElement(Stack, { gap: 47 }, header),
    children,
  ));
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

export type TextProps<D extends Destination = Destination> = PressProps<D> & {
  width?: number;
  children?: ReactNode;
  size?: number;
  align?: "start" | "center" | "end" | "justify";
  maxLines?: number;
  tabularNumbers?: boolean;
}

// Keep text as a host element while checking literal route parameters in JSX.
export const Text = "Text" as "Text" & (<const D extends Destination>(props: TextProps<D>) => ReactElement);

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

export function Button<const D extends Destination>(props: PressProps<D> & {
  children?: ReactNode;
  onLongPress?: () => void;
  disabled?: boolean;
  selected?: boolean;
  icon?: IconAsset;
}) {
  const { href, url, onPress, ...rest } = props;
  return createElement("Button", { ...rest, onPress: pressHandler({ href, url, onPress }) });
}

export { Row } from "./row";
export { Image, Avatar } from "./image";
export { Canvas, Rectangle, CanvasPath, CanvasRange, CanvasText, CanvasIcon } from "./canvas";

export function Field<const D extends Destination>(props: PressProps<D> & {
  label: string;
  disabled?: boolean;
  children?: ReactNode;
}) {
  const { href, url, onPress, ...rest } = props;
  return createElement("Field", { ...rest, onPress: pressHandler({ href, url, onPress }) });
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

/** A titled group of rows, spaced with the theme's section and content gaps. */
export function Section({ title, children }: { title: string; children?: ReactNode }) {
  return createElement(Stack, { gap: "section" },
    createElement("SectionTitle", { title }),
    createElement(Stack, { gap: "content" }, children));
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
export { PlayingScreen, type PlayingScreenProps, type Playback, type TransportControl } from "./playing";
export { ConversationScreen, Message, type ConversationScreenProps, type ConversationMessage, type MessageProps, type MessageAction, type ReplyPreview } from "./conversation";
export { openURL, share } from "./external";
export { LinkPreview, type LinkPreviewData, type LinkPreviewProps } from "./link-preview";
