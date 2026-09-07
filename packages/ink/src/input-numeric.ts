import { TextInput as Input } from "./input-base";
import type { ComponentProps } from "react";
import { createElement } from "./react";

export function TextInput(props: Omit<ComponentProps<typeof Input>, "inputMode" | "action"> & { action?: "search" | "done" }) {
  return createElement(Input, { ...props, inputMode: "numeric", action: props.action ?? "done" });
}
