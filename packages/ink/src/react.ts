import { createElement as element, createContext as context, isValidElement as valid, Fragment as fragment, Component, type ComponentChildren } from "preact";
import { useCallback, useContext as contextValue, useMemo, useRef, useState } from "preact/hooks";
import type * as React from "react";

// Keep the public React declarations while executing Preact core in Ink.
export const createElement = element as unknown as typeof React.createElement;
export const createContext = context as unknown as typeof React.createContext;
export const useContext = contextValue as unknown as typeof React.useContext;
export const isValidElement = valid as unknown as typeof React.isValidElement;
export const Fragment = fragment as unknown as typeof React.Fragment;
export { Children } from "preact/compat";
export { useCallback, useMemo, useRef, useState };
export { Activity, useEffect, useLayoutEffect, useSyncExternalStore } from "./visibility";
export type { ComponentProps, ComponentType, ReactElement, ReactNode } from "react";

// A single core component avoids compat memo's extra function-component layer.
export function memo<P extends object>(render: (props: P) => React.ReactNode) {
  return class extends Component<P> {
    shouldComponentUpdate(next: P) {
      const previous = this.props;
      const keys = Object.keys(next) as (keyof P)[];
      return keys.length !== Object.keys(previous).length || keys.some(key => !Object.is(previous[key], next[key]));
    }
    render() { return render(this.props) as ComponentChildren; }
  } as unknown as (props: P) => React.ReactNode;
}
