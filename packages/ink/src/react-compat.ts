import React from "preact/compat";
import { cloneElement, isValidElement } from "preact";
import { Activity, useEffect, useLayoutEffect, useSyncExternalStore, useImperativeHandle } from "./visibility";
import { useRef, useCallback, useLayoutEffect as commitLayoutEffect } from "preact/hooks";
const compatibleReact = /* @__PURE__ */ Object.assign({}, React, { Activity, cloneElement, isValidElement, useEffect, useLayoutEffect, useSyncExternalStore, useImperativeHandle, useEffectEvent });
export default compatibleReact;
export {
  Children, Component, Fragment, PureComponent, StrictMode, Suspense, SuspenseList,
  createContext, createElement, createFactory, createPortal, createRef,
  findDOMNode, flushSync, forwardRef, hydrate, lazy, memo, render,
  startTransition, unmountComponentAtNode, unstable_batchedUpdates,
  useCallback, useContext, useDebugValue, useDeferredValue,
  useId, useInsertionEffect, useMemo, useReducer,
  useRef, useState, useTransition, version,
} from "preact/compat";
export { cloneElement, isValidElement } from "preact";
export { Activity, useEffect, useLayoutEffect, useSyncExternalStore, useImperativeHandle } from "./visibility";

/** Effect events retain their identity and read the latest committed callback. */
export function useEffectEvent<T extends (...args: never[]) => unknown>(callback: T): T {
  const current = useRef(callback);
  commitLayoutEffect(() => { current.current = callback; });
  return useCallback(((...args: Parameters<T>) => current.current(...args)) as T, []);
}
