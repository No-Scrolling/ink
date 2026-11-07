import { createContext, createElement, type ComponentChildren } from "preact";
import { useContext, useLayoutEffect as layoutEffect, useEffect as effect } from "preact/hooks";
import { useSyncExternalStore as externalStore } from "preact/compat";
import type { DependencyList, EffectCallback, ReactNode } from "react";

const Visibility = createContext(true);

/** Retain the subtree while disconnecting Ink's effects and subscriptions. */
export function Activity({ mode, children }: { mode: "visible" | "hidden"; children?: ReactNode }): ReactNode {
  const parentVisible = useContext(Visibility);
  return createElement(Visibility.Provider, { value: parentVisible && mode === "visible", children: children as ComponentChildren }) as unknown as ReactNode;
}

export function useEffect(callback: EffectCallback, deps?: DependencyList) {
  const visible = useContext(Visibility);
  effect(() => visible ? callback() : undefined, deps ? [visible, ...deps] : undefined);
}

export function useLayoutEffect(callback: EffectCallback, deps?: DependencyList) {
  const visible = useContext(Visibility);
  layoutEffect(() => visible ? callback() : undefined, deps ? [visible, ...deps] : undefined);
}

const disconnected = () => () => {};
export function useSyncExternalStore<T>(subscribe: (callback: () => void) => () => void, getSnapshot: () => T, _getServerSnapshot?: () => T): T {
  const visible = useContext(Visibility);
  return externalStore(visible ? subscribe : disconnected, getSnapshot);
}
