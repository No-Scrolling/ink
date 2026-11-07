import { createElement, type ComponentProps, type ReactElement } from "./react";
import { Button, type StackProps, type TextProps } from "./index";
import type { IconAsset } from "./assets";
import { listText, nativeListRow, nativeListTemplate } from "./native-list";

type SharedProps = { key?: string | number; hidden?: boolean };
type RowProps = {
  Stack: SharedProps & Omit<StackProps, "children">;
  Text: SharedProps & Omit<TextProps, "children" | "href" | "url"> & { text: string };
  Button: SharedProps & Pick<ComponentProps<typeof Button>, "icon" | "disabled" | "selected" | "onPress" | "onLongPress">;
  MediaGridRow: SharedProps;
  MediaCell: SharedProps & {
    src: string; selected?: boolean; dimmed?: boolean; video?: boolean;
    checkIcon?: IconAsset; videoIcon?: IconAsset;
    onPress?: () => void; onLongPress?: () => void;
  };
};
type RowValue = string | number | boolean | undefined | (() => void);
type RowField<Fields, Value extends RowValue> = { readonly read: (fields: Fields) => Value };
type BoundProps<Fields, Props> = { [Name in keyof Props]: (Props[Name] & RowValue) | RowField<Fields, Props[Name] & RowValue> };
type RowNode<Fields> = {
  type: keyof RowProps;
  props: Record<string, RowValue | RowField<Fields, RowValue>>;
  children: RowNode<Fields>[];
};

/** @internal A fixed built-in presentation with React and native list adapters. */
export function fixedListRow<Fields>(build: (
  node: <Kind extends keyof RowProps>(type: Kind, props: BoundProps<Fields, RowProps[NoInfer<Kind>]>, children?: RowNode<Fields>[]) => RowNode<Fields>,
  field: <Value extends RowValue>(read: (fields: Fields) => Value) => RowField<Fields, Value>,
) => RowNode<Fields>) {
  class Field<Value extends RowValue> {
    constructor(readonly read: (fields: Fields) => Value) {}
  }
  const root = build((type, props, children = []) => ({ type, props, children }), read => new Field(read));
  const readers: ((fields: Fields) => RowValue)[] = [];
  const template = nativeListTemplate((node, field) => {
    const visit = (row: RowNode<Fields>): ReturnType<typeof node> => node(row.type,
      Object.fromEntries(Object.entries(row.props).flatMap(([name, value]) => {
        if (name === "key") return [];
        if (value instanceof Field || name.startsWith("on")) {
          const read = value instanceof Field ? value.read : () => value;
          return [[name, field(readers.push(read) - 1)]];
        }
        return [[name, value]];
      })), row.children.map(visit));
    return visit(root);
  });
  function render(row: RowNode<Fields>, fields: Fields): ReactElement | null {
    const props = Object.fromEntries(Object.entries(row.props).map(([name, value]) =>
      [name, value instanceof Field ? value.read(fields) : value]));
    if (props.hidden === true) return null;
    if (row.type === "Text") {
      const { text, ...rest } = props;
      return createElement("Text", rest, listText([text]));
    }
    return createElement(row.type === "Button" ? Button : row.type, props, ...row.children.map(child => render(child, fields)));
  }
  return function<T>(project: (item: T, index: number) => Fields) {
    return nativeListRow((item: T, index: number) => render(root, project(item, index)), (item, index) => {
      const fields = project(item, index);
      return [readers.map(read => read(fields))];
    }, template);
  };
}
