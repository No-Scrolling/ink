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

export interface Computed<T extends Ink.Scalar> {
  readonly value: T;
}

export declare function computed<T extends Ink.Scalar>(
  evaluate: () => T,
): Computed<T>;

export declare function routeParams<
  T extends Readonly<Record<string, Ink.StateScalar | undefined>>,
>(): T;

export type ResourceErrorKind =
  | "unavailable"
  | "permission-denied"
  | "permission-blocked"
  | "location-disabled"
  | "nfc-disabled"
  | "timeout"
  | "protocol"
  | "unexpected";

export interface ResourceError {
  readonly kind: ResourceErrorKind;
  readonly message: string;
  readonly retryable: boolean;
}

interface ReloadableResource {
  reload(): void;
}

export type AsyncResource<T, E = ResourceError> = ReloadableResource &
  (
    | { readonly status: "loading" }
    | { readonly status: "ready"; readonly value: T }
    | { readonly status: "error"; readonly error: E }
  );

type StatusValue = { readonly status: string };

type MatchCases<T extends StatusValue> = {
  readonly [Status in T["status"]]: (
    value: T & { readonly status: Status },
  ) => Ink.Element;
};

export declare function match<T extends StatusValue>(
  value: T,
  cases: MatchCases<T>,
): Ink.Element;

type ReadyValue<T> = Extract<T, { readonly status: "ready" }> extends {
  readonly value: infer Value;
}
  ? Value
  : never;

export type CombinedResource<
  Resources extends Readonly<Record<string, AsyncResource<unknown, unknown>>>,
> =
  | { readonly status: "loading" }
  | {
      readonly status: "ready";
      readonly value: { readonly [Name in keyof Resources]: ReadyValue<Resources[Name]> };
    }
  | { readonly status: "error" };

export declare function all<
  Resources extends Readonly<Record<string, AsyncResource<unknown, unknown>>>,
>(resources: Resources): CombinedResource<Resources>;

/** An opaque source returned by @ink/camera and accepted only by Image. */
export interface CameraImageSource {
  readonly __inkCameraImageSource: never;
}

export declare function state(initial: boolean): Signal<boolean>;
export declare function state(initial: number): Signal<number>;
export declare function state(initial: string): Signal<string>;
export declare function state<T extends Ink.StateScalar>(initial: T): Signal<T>;
export declare function state<T extends ReadonlyArray<Ink.ListItem>>(
  initial: T,
): ListSignal<T[number]>;

export declare function sharedState(key: string, initial: boolean): Signal<boolean>;
export declare function sharedState(key: string, initial: number): Signal<number>;
export declare function sharedState(key: string, initial: string): Signal<string>;
export declare function sharedState<T extends Ink.StateScalar>(
  key: string,
  initial: T,
): Signal<T>;
export declare function sharedState<T extends ReadonlyArray<Ink.ListItem>>(
  key: string,
  initial: T,
): ListSignal<T[number]>;

export declare function persistedState(key: string, initial: boolean): Signal<boolean>;
export declare function persistedState(key: string, initial: number): Signal<number>;
export declare function persistedState(key: string, initial: string): Signal<string>;
export declare function persistedState<T extends Ink.StateScalar>(
  key: string,
  initial: T,
): Signal<T>;
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
  type StateScalar = Scalar | null;
  type ListItem = StateScalar | { readonly [key: string]: ListItem };
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
    | {
        href:
          | string
          | {
              readonly path: string;
              readonly params?: Readonly<Record<string, StateScalar>>;
            };
        onPress?: never;
      }
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
    src: string | CameraImageSource;
    fallback?: string;
    bleed?: boolean;
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
