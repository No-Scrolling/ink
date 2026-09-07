import { createElement, useState } from "./react";

export function TextInput(props: {
  value: string;
  onChange: (value: string) => void;
  onSubmit?: (value: string) => void;
  placeholder?: string;
  autoFocus?: boolean;
  action?: "return" | "search" | "done";
  inputMode?: "text" | "numeric";
  prefix?: string;
  suffix?: string;
}) {
  const [eventCount, setEventCount] = useState(0);
  const nativeProps = {
    ...props,
    eventCount,
    onChange(value: string, count: number) {
      setEventCount(count);
      props.onChange(value);
    },
  };
  return createElement("TextInput", nativeProps);
}

