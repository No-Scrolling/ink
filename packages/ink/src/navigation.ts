import type { IconAsset } from "./assets";
import {
  Activity, Children, createContext, createElement, isValidElement, useContext,
  useLayoutEffect, useMemo, useRef, useState, type ReactElement, type ReactNode,
} from "react";
import { onNativeMessage } from "./native";

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type Params = Record<string, Json>;
export type Destination = string | { path: string; params?: Params };
type Entry = { key: number; path: string; params: Params; unavailable?: boolean };
type NavigationAction = { type: "back" } | { type: "push" | "replace"; destination: Destination };
let dispatch: ((action: NavigationAction) => void) | undefined;
let openNativeRoute: ((destination: Destination) => void) | undefined;
let pendingNativeRoute: Destination | undefined;
onNativeMessage("navigate", message => {
  if (typeof message.path !== "string" || !message.path.startsWith("/")) return;
  const params = message.params;
  if (params !== undefined && (typeof params !== "object" || params === null || Array.isArray(params))) return;
  const destination: Destination = { path: message.path, params: params as Params | undefined };
  if (openNativeRoute) openNativeRoute(destination);
  else pendingNativeRoute = destination;
});
const RouteContext = createContext<Params | undefined>(undefined);

function send(action: NavigationAction) {
  if (!dispatch) throw new Error("Navigation requires a mounted Navigator");
  dispatch(action);
}

export function navigate(destination: Destination) { send({ type: "push", destination }); }
export function replace(destination: Destination) { send({ type: "replace", destination }); }
export function back() { send({ type: "back" }); }

export function useRouteParams<T extends object = Params>(decode?: (value: unknown) => T): T {
  const params = useContext(RouteContext);
  if (!params) throw new Error("useRouteParams requires a Route");
  return decode ? decode(params) : params as T;
}

type RouteProps = { path: string; children: ReactNode };
export function Route(_props: RouteProps): ReactNode {
  throw new Error("Route must be a direct child of Navigator");
}

export function Navigator({ children }: { children: ReactNode }) {
  const routes = useMemo(() => {
    const routes = new Map<string, ReactNode>();
    Children.forEach(children, child => {
      if (!isValidElement<RouteProps>(child) || child.type !== Route) {
        throw new Error("Navigator children must be Routes");
      }
      if (routes.has(child.props.path)) throw new Error(`Duplicate route: ${child.props.path}`);
      routes.set(child.props.path, child.props.children);
    });
    if (!routes.has("/")) throw new Error("Navigator requires a / route");
    return routes;
  }, [children]);
  const [entries, setEntries] = useState<Entry[]>([{ key: 0, path: "/", params: {} }]);
  const nextKey = useRef(1);
  useLayoutEffect(() => {
    if (dispatch) throw new Error("An Ink app can mount one Navigator");
    dispatch = action => {
      if (action.type === "back") {
        setEntries(current => current.length > 1 ? current.slice(0, -1) : current);
        return;
      }
      const target = typeof action.destination === "string"
        ? { path: action.destination, params: {} } : action.destination;
      if (!routes.has(target.path)) throw new Error(`Unknown route: ${target.path}`);
      const entry = { key: nextKey.current++, path: target.path, params: target.params ?? {} };
      setEntries(current => action.type === "replace" ? [...current.slice(0, -1), entry] : [...current, entry]);
    };
    openNativeRoute = destination => {
      const path = typeof destination === "string" ? destination : destination.path;
      if (routes.has(path)) dispatch?.({ type: "push", destination });
      else setEntries(current => [...current, {
        key: nextKey.current++, path, params: {}, unavailable: true,
      }]);
    };
    if (pendingNativeRoute !== undefined) {
      const path = pendingNativeRoute;
      pendingNativeRoute = undefined;
      openNativeRoute(path);
    }
    return () => { dispatch = undefined; openNativeRoute = undefined; };
  }, [routes]);
  const nativeProps = { onBack: entries.length > 1 ? back : undefined };
  return createElement("Navigator", nativeProps,
    entries.map((entry, index) => createElement(Activity, {
      key: entry.key,
      mode: index === entries.length - 1 ? "visible" : "hidden",
      children: createElement(RouteContext.Provider, { value: entry.params }, entry.unavailable
        ? createElement("Screen", { title: "Page unavailable" },
          createElement("Text", null, "This notification links to a page that is no longer available."),
          createElement("Button", { onPress: back }, "Go back"))
        : routes.get(entry.path)),
    })),
  );
}

type TabProps = { id?: string; icon: IconAsset; children: ReactNode };
export function Tab(_props: TabProps): ReactNode {
  throw new Error("Tab must be a direct child of Tabs");
}

export function Tabs({ children }: { children: ReactNode }) {
  const tabs: ReactElement<TabProps>[] = [];
  const ids = new Set<string>();
  Children.forEach(children, child => {
    if (!isValidElement<TabProps>(child) || child.type !== Tab) throw new Error("Tabs children must be Tab components");
    const id = child.props.id ?? child.props.icon;
    if (ids.has(id)) throw new Error(`Duplicate tab: ${id}`);
    ids.add(id);
    tabs.push(child);
  });
  if (!tabs.length) throw new Error("Tabs requires at least one Tab");
  const first = tabs[0].props.id ?? tabs[0].props.icon;
  const [selected, setSelected] = useState(first);
  const selectedIndex = tabs.findIndex(tab => (tab.props.id ?? tab.props.icon) === selected);
  const active = selectedIndex < 0 ? 0 : selectedIndex;
  useLayoutEffect(() => {
    if (selectedIndex < 0) setSelected(first);
  }, [first, selectedIndex]);
  const nativeProps = { active };
  return createElement("Tabs", nativeProps, tabs.map((tab, index) => {
    const id = tab.props.id ?? tab.props.icon;
    const nativeProps = { key: id, icon: tab.props.icon, onPress: () => setSelected(id) };
    return createElement("Tab", nativeProps,
      createElement(Activity, { mode: index === active ? "visible" : "hidden", children: tab.props.children }));
  }));
}
