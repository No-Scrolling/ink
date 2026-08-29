import type { Ink } from "./index";

export function jsx(type: unknown, props: unknown): Ink.Element;
export function jsxs(type: unknown, props: unknown): Ink.Element;
export const Fragment: unique symbol;

export namespace JSX {
  type Element = Ink.Element;

  interface IntrinsicElements {
    Screen: Ink.ScreenProps;
    Column: Ink.ColumnProps;
    Text: Ink.TextProps;
    Button: Ink.ButtonProps;
  }
}
