import { TextInput as Input } from "./input-base";
import type { ComponentProps } from "react";
import { createElement } from "./react";

export function TextInput(props: ComponentProps<typeof Input>) {
  return createElement(Input, props);
}
