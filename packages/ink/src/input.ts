import { createElement, useState } from "./react";
import type { IconAsset } from "./assets";
import type { HitAreaProps } from "./press";
import type { TextInputAppearance } from "./theme";

export function TextInput(props: HitAreaProps & {
  value: string;
  onChange: (value: string) => void;
  onSubmit?: (value: string) => void;
  placeholder?: string;
  autoFocus?: boolean;
  action?: "return" | "search" | "done";
  inputMode?: "text" | "numeric";
  prefix?: string;
  suffix?: string;
  label?: string;
  description?: string;
  error?: string;
  invalid?: boolean;
  disabled?: boolean;
  readOnly?: boolean;
  clearable?: boolean;
  leadingIcon?: IconAsset;
  minLines?: number;
  maxLines?: number;
  /** Overrides the theme. Sizes here use Ink units, like `Text` and `Icon`. */
  style?: TextInputAppearance;
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
  const input = createElement("TextInput", nativeProps);
  if (!props.label && !props.description && !props.error) return input;
  return createElement("Stack", { gap: 6 },
    props.label && createElement("Text", { size: 16 }, props.label),
    input,
    (props.error || props.description) && createElement("Text", { size: 16 }, props.error || props.description),
  );
}
