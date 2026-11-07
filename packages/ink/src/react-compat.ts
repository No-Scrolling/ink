import React from "preact/compat";
import { Activity } from "./visibility";
import { useRef, useLayoutEffect, useCallback } from "preact/hooks";
const compatibleReact = /* @__PURE__ */ Object.assign({}, React, { Activity, useEffectEvent });
export default compatibleReact;
export {
  Children, Component, Fragment, PureComponent, StrictMode, Suspense, SuspenseList,
  cloneElement, createContext, createElement, createFactory, createPortal, createRef,
  findDOMNode, flushSync, forwardRef, hydrate, isValidElement, lazy, memo, render,
  startTransition, unmountComponentAtNode, unstable_batchedUpdates,
  useCallback, useContext, useDebugValue, useDeferredValue, useEffect,
  useId, useImperativeHandle, useInsertionEffect, useLayoutEffect, useMemo, useReducer,
  useRef, useState, useSyncExternalStore, useTransition, version,
} from "preact/compat";
export { Activity } from "./visibility";

/** Effect events retain their identity and read the latest committed callback. */
export function useEffectEvent<T extends (...args: never[]) => unknown>(callback: T): T {
  const current = useRef(callback);
  useLayoutEffect(() => { current.current = callback; });
  return useCallback(((...args: Parameters<T>) => current.current(...args)) as T, []);
}
