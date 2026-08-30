import type { Ink } from "./index";

export function jsx(type: unknown, props: unknown): Ink.Element;
export function jsxs(type: unknown, props: unknown): Ink.Element;
export const Fragment: unique symbol;

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
  }
}
