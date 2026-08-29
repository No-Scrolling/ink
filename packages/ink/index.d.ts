export interface Signal<T> {
  readonly value: T;
  set(value: T): void;
}

export declare function state<T>(initial: T): Signal<T>;

export declare function Screen(props: Ink.ScreenProps): Ink.Element;
export declare function Column(props: Ink.ColumnProps): Ink.Element;
export declare function Text(props: Ink.TextProps): Ink.Element;
export declare function Button(props: Ink.ButtonProps): Ink.Element;

export namespace Ink {
  interface Element {}

  interface Children {
    children?: Element | string | number | Array<Element | string | number>;
  }

  interface ScreenProps extends Children {}

  interface ColumnProps extends Children {
    gap?: number;
  }

  interface TextProps extends Children {
    size?: number;
  }

  interface ButtonProps extends Children {
    onPress: () => void;
  }
}

export namespace JSX {
  type Element = Ink.Element;

  interface IntrinsicElements {
    Screen: Ink.ScreenProps;
    Column: Ink.ColumnProps;
    Text: Ink.TextProps;
    Button: Ink.ButtonProps;
  }
}
