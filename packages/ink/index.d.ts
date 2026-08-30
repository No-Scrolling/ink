export interface Signal<T> {
  readonly value: T;
  set(value: T): void;
}

export interface ListSignal<T> {
  readonly value: ReadonlyArray<T>;
  set(value: ReadonlyArray<T>): void;
  append(value: T): void;
  remove(value: T): void;
  replace(value: T, replacement: T): void;
  clear(): void;
}

export declare function state(initial: boolean): Signal<boolean>;
export declare function state(initial: number): Signal<number>;
export declare function state(initial: string): Signal<string>;
export declare function state<T extends ReadonlyArray<Ink.ListItem>>(
  initial: T,
): ListSignal<T[number]>;

export declare function sharedState(key: string, initial: boolean): Signal<boolean>;
export declare function sharedState(key: string, initial: number): Signal<number>;
export declare function sharedState(key: string, initial: string): Signal<string>;
export declare function sharedState<T extends ReadonlyArray<Ink.ListItem>>(
  key: string,
  initial: T,
): ListSignal<T[number]>;

export declare function persistedState(key: string, initial: boolean): Signal<boolean>;
export declare function persistedState(key: string, initial: number): Signal<number>;
export declare function persistedState(key: string, initial: string): Signal<string>;
export declare function persistedState<T extends ReadonlyArray<Ink.ListItem>>(
  key: string,
  initial: T,
): ListSignal<T[number]>;
export declare function back(): void;

export declare function Screen(props: Ink.ScreenProps): Ink.Element;
export declare function Stack(props: Ink.StackProps): Ink.Element;
export declare function Text(props: Ink.TextProps): Ink.Element;
export declare function TextInput(props: Ink.TextInputProps): Ink.Element;
export declare function Button(props: Ink.ButtonProps): Ink.Element;
export declare function SelectorButton(props: Ink.SelectorButtonProps): Ink.Element;
export declare function Icon(props: Ink.IconProps): Ink.Element;
export declare function Image(props: Ink.ImageProps): Ink.Element;
export declare function Toggle(props: Ink.ToggleProps): Ink.Element;
export declare function Tabs(props: Ink.TabsProps): Ink.Element;
export declare function Tab(props: Ink.TabProps): Ink.Element;
export declare function Navigator(props: Ink.NavigatorProps): Ink.Element;
export declare function Route(props: Ink.RouteProps): Ink.Element;

export namespace Ink {
  interface Element {}

  type Scalar = boolean | number | string;
  type ListItem = Scalar | { readonly [key: string]: ListItem };
  type Child = Element | string | number | false | null;
  type Children = Child | ReadonlyArray<Child>;
  type TextContent = Scalar | ReadonlyArray<Scalar>;
  type Alignment = "start" | "center" | "end" | "stretch";
  type Justification = "start" | "center" | "end" | "space-between";
  type TextAlignment = "start" | "center" | "end";
  type Tone = "primary" | "muted";

  interface ScreenProps {
    children?: Children;
    title?: string;
    centered?: boolean;
  }

  interface StackProps {
    children?: Children;
    axis?: "vertical" | "horizontal";
    gap?: number;
    align?: Alignment;
    justify?: Justification;
  }

  interface TextProps {
    children: Children;
    size?: number;
    align?: TextAlignment;
  }

  interface TextInputProps {
    placeholder: string;
    value: string;
    onChange(value: string): void;
    action?: "search" | "return" | "done";
  }

  type PressProps =
    | { href: string; onPress?: never }
    | { href?: never; onPress?: () => void };

  type ButtonProps = {
    children: TextContent;
    icon?: string;
    underline?: boolean;
  } & PressProps;

  type SelectorButtonProps = {
    children: TextContent;
    label: string;
  } & PressProps;

  interface IconProps {
    name: string;
    size?: number;
    tone?: Tone;
  }

  interface ImageProps {
    src: string;
    width: number;
    height: number;
    fit?: "cover" | "contain";
  }

  interface ToggleProps {
    label: string;
    value: boolean;
    onChange: () => void;
  }

  interface TabsProps {
    children: Element | ReadonlyArray<Element>;
    value: number;
  }

  interface TabProps {
    children: Element;
    icon: string;
    onPress: () => void;
  }

  interface NavigatorProps {
    children: Element | ReadonlyArray<Element>;
  }

  interface RouteProps {
    children: Element;
    path: string;
  }
}

export namespace JSX {
  type Element = Ink.Element;

  interface ElementChildrenAttribute {
    children: {};
  }

  interface IntrinsicElements {
    Screen: Ink.ScreenProps;
    Stack: Ink.StackProps;
    Text: Ink.TextProps;
    TextInput: Ink.TextInputProps;
    Button: Ink.ButtonProps;
    SelectorButton: Ink.SelectorButtonProps;
    Icon: Ink.IconProps;
    Image: Ink.ImageProps;
    Toggle: Ink.ToggleProps;
    Tabs: Ink.TabsProps;
    Tab: Ink.TabProps;
    Navigator: Ink.NavigatorProps;
    Route: Ink.RouteProps;
  }
}
