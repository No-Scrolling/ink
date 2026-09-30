import { Activity, createElement, useMemo, type ComponentType, type ReactNode } from "react";
import { back, LayoutContext, NavigationStack, RouteContext, type Destination, type Entry } from "./navigation";

export type FileRoute = { name: string; path: string; component: ComponentType };
export type FileLayout = { name: string; layout?: ComponentType; pages: FileRoute[]; children: FileLayout[] };

function paths(group: FileLayout): string[] {
  return [...group.pages.map(page => page.path), ...group.children.flatMap(paths)];
}

function LayoutBranch({ group, entries, active, selectTab, root = false }: {
  group: FileLayout;
  entries: Entry[];
  active: Entry;
  selectTab: (path: string) => void;
  root?: boolean;
}) {
  const groupPaths = useMemo(() => new Set(paths(group)), [group]);
  const local = entries.filter(entry => groupPaths.has(entry.path) && !entry.unavailable && !entry.page);
  const last = local.at(-1);
  if (!root && !last) return null;
  const renderPage = (route: FileRoute, entry: Entry) => (
    <Activity key={entry.key} mode={entry.key === active.key ? "visible" : "hidden"}>
      <RouteContext.Provider value={entry.params}><route.component /></RouteContext.Provider>
    </Activity>
  );
  const pages = new Map(group.pages.map(route => {
    const entry = local.filter(entry => entry.path === route.path).at(-1);
    return [route.name, {
      path: route.path,
      page: <RouteContext.Provider value={entry?.params ?? {}}><route.component /></RouteContext.Provider>,
    }];
  }));
  const fallback = root && entries.filter(entry => entry.page || entry.unavailable).map(entry => (
    <Activity key={entry.key} mode={entry.key === active.key ? "visible" : "hidden"}>
      <RouteContext.Provider value={entry.params}>
        {entry.page ?? createElement("Screen", { title: "Page unavailable" },
          createElement("Text", null, "This notification links to a page that is no longer available."),
          createElement("Button", { onPress: back }, "Go back"))}
      </RouteContext.Provider>
    </Activity>
  ));
  const content = <>
    {group.pages.flatMap(route => local.filter(entry => entry.path === route.path).map(entry => renderPage(route, entry)))}
    {group.children.map(child => <LayoutBranch key={child.name} group={child} entries={entries} active={active} selectTab={selectTab} />)}
    {fallback}
  </>;
  const ordinaryPages = (tabPaths: readonly string[]) => <>
    {group.pages.flatMap(route => local.filter(entry =>
      entry.path === route.path && entry.key !== 0 && entry.tabGroup !== group && !tabPaths.includes(entry.path)).map(entry => renderPage(route, entry)))}
    {fallback}
  </>;
  const value = {
    content, pages, group, ordinaryPages, selectTab,
    // The implicit startup entry belongs to its tabs before registration commits.
    tabPath: local.find(entry => entry.tabGroup === group || entry.key === 0)?.path,
    directories: group.children.map(child => child.name),
    activePath: root && (active.page || active.unavailable) ? active.path : last?.path ?? active.path,
  };
  return <Activity mode={root || groupPaths.has(active.path) && !active.page && !active.unavailable ? "visible" : "hidden"}>
    <LayoutContext.Provider value={value}>
      <RouteContext.Provider value={last?.params ?? active.params}>
        {group.layout ? createElement(group.layout) : content}
      </RouteContext.Provider>
    </LayoutContext.Provider>
  </Activity>;
}

export function FileNavigator({ tree, initialDestination }: { tree: FileLayout; initialDestination?: Destination }) {
  const routes = useMemo(() => new Set(paths(tree)), [tree]);
  return <NavigationStack routes={routes} initialDestination={initialDestination} renderEntries={(entries, selectTab) => (
    <LayoutBranch group={tree} entries={entries} active={entries[entries.length - 1]} selectTab={selectTab} root />
  )} />;
}
