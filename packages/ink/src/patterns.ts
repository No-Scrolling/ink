import { createElement, type ReactNode } from "./react";
import { Button, Stack, Text } from "./index";

export type SettingsChoice<Value extends string> = {
  value: Value;
  label: string;
};

export type SettingsChoicesProps<Value extends string> = {
  options: readonly SettingsChoice<Value>[];
  value: Value;
  onChange: (value: Value) => void;
  disabled?: boolean;
};

export function SettingsChoices<Value extends string>({ options, value, onChange, disabled }: SettingsChoicesProps<Value>) {
  return createElement(Stack, { gap: 47 }, options.map(option => createElement(Button, {
    key: option.value,
    selected: option.value === value,
    disabled,
    onPress: () => onChange(option.value),
  }, option.label)));
}

export function LoadingState({ label = "Loading…" }: { label?: string }) {
  return createElement("ScreenState", { message: label });
}

type StateAction = { label: string; onPress: () => void; disabled?: boolean };

export type EmptyStateProps = {
  title: string;
  description?: string;
  action?: StateAction;
};

export function EmptyState({ title, description, action }: EmptyStateProps) {
  return createElement("ScreenState", {
    message: description ? `${title}\n\n${description}` : title,
    retryLabel: action?.label.toUpperCase(),
    onRetry: action?.onPress,
    disabled: action?.disabled,
  });
}

export type ErrorStateProps = {
  message: string;
  onRetry: () => void;
  retryLabel?: string;
  disabled?: boolean;
};

export function ErrorState({ message, onRetry, retryLabel = "Try again", disabled }: ErrorStateProps) {
  return createElement("ScreenState", { message, retryLabel: retryLabel.toUpperCase(), onRetry, disabled });
}

export type ConfirmationProps = {
  title: string;
  children: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  pending?: boolean;
  pendingLabel?: string;
};

export function Confirmation({ title, children, confirmLabel, onConfirm, pending = false, pendingLabel = "Working…" }: ConfirmationProps) {
  return createElement("Confirmation", {
    title,
    confirmLabel: (pending ? pendingLabel : confirmLabel).toUpperCase(),
    onConfirm,
    pending,
  }, createElement(Text, { size: 18, align: "start" }, children));
}
