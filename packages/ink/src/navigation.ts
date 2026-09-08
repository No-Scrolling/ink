import type { IconAsset } from "./assets";
import {
  Activity, Children, Fragment, createContext, createElement, isValidElement, useContext,
  useLayoutEffect, useMemo, useRef, useState, type ReactElement, type ReactNode,
} from "react";
import { onNativeMessage } from "./native";
import { comparePaths, matchPath, parameterNames, type PathParams } from "./route-path";

type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
type Params = Record<string, Json>;
export type Destination = string | { path: string; params?: Params };
export type Entry = { key: number; path: string; params: Params; tabGroup?: object; unavailable?: boolean; page?: ReactElement };
type NavigationAction = { type: "back" } | { type: "push" | "replace"; destination: Destination } | { type: "present"; page: ReactElement };
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
export const RouteContext = createContext<Params | undefined>(undefined);
const TabRoutesContext = createContext<((group: object, paths: readonly string[], ownsStart: boolean) => void) | null>(null);

function send(action: NavigationAction) {
  if (!dispatch) throw new Error("Navigation is unavailable before the app mounts");
  dispatch(action);
}

type CheckedDestination<D extends Destination> = D extends string
  ? keyof PathParams<D> extends never ? D : never
  : D extends { path: infer Path extends string }
    ? keyof PathParams<Path> extends never ? D : D & { params: { [Key in keyof PathParams<Path>]: string | number } }
    : never;

export function navigate<const D extends Destination>(destination: D & CheckedDestination<D>) { send({ type: "push", destination }); }
export function replace<const D extends Destination>(destination: D & CheckedDestination<D>) { send({ type: "replace", destination }); }
export function back() { send({ type: "back" }); }

export function presentPage(page: ReactElement) { send({ type: "present", page }); }

export function useRouteParams<Path extends string>(path: Path): PathParams<Path>;
export function useRouteParams<T extends object = Params>(decode?: (value: unknown) => T): T;
export function useRouteParams(decode?: string | ((value: unknown) => object)): object {
  const params = useContext(RouteContext);
  if (!params) throw new Error("useRouteParams must be used inside a page");
  if (typeof decode === "function") return decode(params);
  if (typeof decode === "string") {
    for (const name of parameterNames(decode)) {
      if (typeof params[name] !== "string" || !params[name]) throw new Error(`Missing route parameter: ${name}`);
    }
  }
  return params;
}

export function NavigationStack({ routes, renderEntries }: {
  routes: ReadonlySet<string>;
  renderEntries: (entries: Entry[], selectTab: (path: string) => void) => ReactNode;
}) {
  const [entries, setEntries] = useState<Entry[]>([{ key: 0, path: "/", params: {} }]);
  const nextKey = useRef(1);
  // Tab membership belongs to the route table, even while its layout is covered or unmounted.
  const tabGroups = useMemo(() => new Map<object, readonly string[]>(), [routes]);
  const registerTabs = useMemo(() => (group: object, paths: readonly string[], ownsStart: boolean) => {
    for (const path of paths) {
      if (!routes.has(path)) throw new Error(`Unknown tab route: ${path}`);
      if ([...tabGroups].some(([owner, members]) => owner !== group && members.includes(path))) {
        throw new Error(`Route belongs to more than one tab layout: ${path}`);
      }
    }
    const previous = tabGroups.get(group);
    if (previous?.length === paths.length && previous.every((path, index) => path === paths[index])) return;
    tabGroups.set(group, paths);
    // Coalesce visits made before the tab layout registered.
    setEntries(current => {
      const members = current.filter(entry => !entry.page && !entry.unavailable && (
        entry.tabGroup === group || paths.includes(entry.path) || ownsStart && entry.key === 0
      ));
      if (!members.length) return current;
      const first = members[0];
      const last = members[members.length - 1];
      const retained = paths.includes(last.path);
      return [...current.slice(0, current.indexOf(first)), { ...first, tabGroup: group, path: retained ? last.path : paths[0], params: retained ? last.params : {} }, ...current.slice(current.indexOf(last) + 1)];
    });
  }, [routes, tabGroups]);
  const resolve = useMemo(() => (destination: Destination) => {
    const target = typeof destination === "string" ? { path: destination, params: {} } : destination;
    if (routes.has(target.path)) {
      const params = { ...target.params };
      for (const name of parameterNames(target.path)) {
        const value = params[name];
        if ((typeof value !== "string" && typeof value !== "number") || String(value) === "") throw new Error(`Missing route parameter: ${name}`);
        params[name] = String(value);
      }
      return { path: target.path, params };
    }
    for (const pattern of [...routes.keys()].sort(comparePaths)) {
      const params = matchPath(pattern, target.path);
      if (params) return { path: pattern, params: { ...target.params, ...params } };
    }
    throw new Error(`Unknown route: ${target.path}`);
  }, [routes]);
  useLayoutEffect(() => {
    if (dispatch) throw new Error("An Ink app can mount one router");
    dispatch = action => {
      if (action.type === "back") {
        setEntries(current => current.length > 1 ? current.slice(0, -1) : current);
        return;
      }
      if (action.type === "present") {
        const key = nextKey.current++;
        setEntries(current => [...current, { key, path: "", params: {}, page: action.page }]);
        return;
      }
      const target = resolve(action.destination);
      const entry = { key: nextKey.current++, ...target };
      setEntries(current => {
        const group = [...tabGroups].find(([, paths]) => paths.includes(target.path))?.[0];
        const index = group ? current.findIndex(entry => entry.tabGroup === group) : -1;
        if (index >= 0) return [...current.slice(0, index), { ...current[index], path: entry.path, params: entry.params }];
        const next = { ...entry, tabGroup: group };
        return action.type === "replace" ? [...current.slice(0, -1), next] : [...current, next];
      });
    };
    openNativeRoute = destination => {
      const path = typeof destination === "string" ? destination : destination.path;
      try { resolve(destination); } catch {
        setEntries(current => [...current, {
          key: nextKey.current++, path, params: {}, unavailable: true,
        }]);
        return;
      }
      dispatch?.({ type: "push", destination });
    };
    if (pendingNativeRoute !== undefined) {
      const path = pendingNativeRoute;
      pendingNativeRoute = undefined;
      openNativeRoute(path);
    }
    return () => { dispatch = undefined; openNativeRoute = undefined; };
  }, [routes, tabGroups, resolve]);
  const nativeProps = { onBack: entries.length > 1 ? back : undefined };
  return createElement(TabRoutesContext.Provider, { value: registerTabs }, createElement("Navigator", nativeProps,
    renderEntries(entries, navigate),
  ));
}

export const LayoutContext = createContext<{
  content: ReactNode;
  pages: ReadonlyMap<string, { path: string; page: ReactNode }>;
  directories: readonly string[];
  activePath: string;
  group: object;
  tabPath?: string;
  ordinaryPages: (tabPaths: readonly string[]) => ReactNode;
  selectTab: (path: string) => void;
} | null>(null);

export function Slot() {
  const layout = useContext(LayoutContext);
  if (!layout) throw new Error("Slot belongs in an app/_layout.tsx file");
  return layout.content;
}

type TabScreenProps = { name: string; icon: IconAsset };
function TabScreen(_props: TabScreenProps): ReactNode {
  throw new Error("Tabs.Screen must be a direct child of Tabs");
}

type TabActionProps = { icon: IconAsset; onPress: () => void };
function TabAction(_props: TabActionProps): ReactNode {
  throw new Error("Tabs.Action must be a direct child of Tabs");
}

function actionItem(action: ReactElement<TabActionProps>) {
  return createElement("Tab", {
    key: `action:${action.key}`, icon: action.props.icon, onPress: action.props.onPress,
  });
}

export function Tabs({ children }: { children: ReactNode }) {
  const layout = useContext(LayoutContext);
  const registerTabs = useContext(TabRoutesContext);
  if (!layout) throw new Error("Tabs.Screen belongs in an app/_layout.tsx file");
  if (layout.directories.length) throw new Error(`Tab layouts support neighbouring page files only. Move detail directories outside this tab group: ${layout.directories.join(", ")}`);
  const names = new Set<string>();
  const items: ({ action: ReactElement<TabActionProps> } | { name: string; icon: IconAsset; path: string; page: ReactNode })[] = [];
  Children.toArray(children).forEach(child => {
    if (isValidElement<TabActionProps>(child) && child.type === TabAction) {
      items.push({ action: child });
      return;
    }
    if (!isValidElement<TabScreenProps>(child) || child.type !== TabScreen) throw new Error("Use Tabs.Screen or Tabs.Action in a file-based tab layout");
    const { name, icon } = child.props;
    if (names.has(name)) throw new Error(`Duplicate tab: ${name}`);
    names.add(name);
    const page = layout.pages.get(name);
    if (!page) throw new Error(`No neighbouring page for tab: ${name}`);
    if (parameterNames(page.path).length) throw new Error(`Tabs need static paths. Move ${page.path} outside the tab group`);
    items.push({ name, icon, ...page });
  });
  const tabs = items.filter(item => "path" in item);
  if (!tabs.length) throw new Error("Tabs requires at least one Tabs.Screen");
  const paths = tabs.map(tab => tab.path);
  const ownsStart = [...layout.pages.values()].some(page => page.path === "/");
  useLayoutEffect(() => {
    registerTabs?.(layout.group, paths, ownsStart);
  }, [layout.group, JSON.stringify(paths), ownsStart, registerTabs]);
  const selected = tabs.find(tab => tab.path === (layout.tabPath ?? layout.activePath)) ?? tabs[0];
  const active = items.indexOf(selected);
  const visible = paths.includes(layout.activePath) || layout.tabPath === layout.activePath;
  const bar = createElement("Tabs", { active }, items.map((tab, index) => {
    if ("action" in tab) return actionItem(tab.action);
    return createElement("Tab", {
      key: tab.name, icon: tab.icon, onPress: () => layout.selectTab(tab.path),
    }, createElement(Activity, { mode: index === active ? "visible" : "hidden", children: tab.page }));
  }));
  return createElement(Fragment, null,
    createElement(Activity, { mode: visible ? "visible" : "hidden", children: bar }),
    layout.ordinaryPages(paths));
}

Tabs.Screen = TabScreen;
Tabs.Action = TabAction;
