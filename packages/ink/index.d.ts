export interface Signal<T> {
  readonly value: T;
  set(value: T): void;
}

export declare function state(initial: boolean): Signal<boolean>;
export declare function state(initial: number): Signal<number>;

export declare function Screen(props: Ink.ScreenProps): Ink.Element;
export declare function Stack(props: Ink.StackProps): Ink.Element;
export declare function Text(props: Ink.TextProps): Ink.Element;
export declare function TextInput(props: Ink.TextInputProps): Ink.Element;
export declare function Button(props: Ink.ButtonProps): Ink.Element;
export declare function Icon(props: Ink.IconProps): Ink.Element;
export declare function Image(props: Ink.ImageProps): Ink.Element;
export declare function Toggle(props: Ink.ToggleProps): Ink.Element;
export declare function Tabs(props: Ink.TabsProps): Ink.Element;
export declare function Tab(props: Ink.TabProps): Ink.Element;
export declare function Navigator(props: Ink.NavigatorProps): Ink.Element;
export declare function Route(props: Ink.RouteProps): Ink.Element;

export namespace Ink {
  interface Element {}

  type Child = Element | string | number;
  type Children = Child | ReadonlyArray<Child>;
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
  }

  type ButtonProps = {
    children: string;
    icon?: string;
    underline?: boolean;
  } & (
    | { href: string; onPress?: never }
    | { href?: never; onPress?: () => void }
  );

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
    Icon: Ink.IconProps;
    Image: Ink.ImageProps;
    Toggle: Ink.ToggleProps;
    Tabs: Ink.TabsProps;
    Tab: Ink.TabProps;
    Navigator: Ink.NavigatorProps;
    Route: Ink.RouteProps;
  }
}
