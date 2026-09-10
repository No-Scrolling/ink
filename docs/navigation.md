---
title: "Navigation"
description: "Connect pages, tabs and actions."
---

## Pages

Create pages in `app/`. Each page exports a React component as its default export. Ink generates the entry point in `.ink/`.

```text
app/
├── _layout.tsx             # Optional shared providers
├── (tabs)/
│   ├── _layout.tsx         # Tab order and icons
│   ├── index.tsx           # /
│   ├── locations.tsx       # /locations
│   └── settings.tsx        # /settings
├── search.tsx              # /search
└── settings/
    └── units.tsx           # /settings/units
```

`index.tsx` uses its directory's path. Parenthesised directories group pages without adding a path segment. For example, `app/(tabs)/index.tsx` and `app/index.tsx` both resolve to `/`; keep only one. Files beginning with `_` are ignored except `_layout.tsx`.

Keep supporting code outside `app/`; see [project structure](/project-structure).

## Dynamic pages

Use `[id]` for a dynamic path segment, such as `app/album/[id].tsx` or `app/playlist/[id]/edit.tsx`:

```tsx
navigate({ path: "/album/[id]", params: { id: album.id } });

// app/album/[id].tsx
const { id } = useRouteParams("/album/[id]"); // id is a string
```

TypeScript checks the required parameters in `navigate()` and `replace()` using the path string. Ink also checks them at runtime and converts numeric IDs to strings. A path such as `/album/123` matches `app/album/[id].tsx`; encoded segments are decoded. Handle IDs that refer to missing data in your page.

### Route matching

Static pages take priority over dynamic pages: `/playlist/new` opens `new.tsx` rather than `[id].tsx`. Duplicate route patterns and repeated parameter names are errors. Catch-all and optional segments are not supported.

## Layouts

A layout wraps the pages beneath it. Use `Slot` to render them:

```tsx
// app/_layout.tsx
import { Slot } from "ink";
import { AccountProvider } from "../components/AccountProvider";

export default function Layout() {
  return <AccountProvider><Slot /></AccountProvider>;
}
```

The root layout stays mounted while navigating. Nested layouts share state across their pages. Ink removes a nested layout when none of its pages remain on the back stack. Covered layouts keep state and pause effects.

Layouts are optional. Add one when pages need shared state or tabs.

## Tabs

A tab layout declares neighbouring pages by name:

```tsx
// app/(tabs)/_layout.tsx
import { Tabs } from "ink";
import { homeFilled, settingsFilled } from "ink/icons";

export default function Layout() {
  return <Tabs>
    <Tabs.Screen name="index" icon={homeFilled} />
    <Tabs.Screen name="settings" icon={settingsFilled} />
  </Tabs>;
}
```

Tab layouts contain neighbouring page files only; nested directories and nested tab navigation are not supported. Put detail pages outside the tab group so they open above the tab bar. Ink reports unsupported directories when the layout renders.

### Show, hide and reorder tabs

Declare the tabs you want to show, using React conditions or array mapping. A neighbouring page omitted from `Tabs.Screen` is still reachable: navigating to it opens a separate page without the tab bar, and Back returns to the previous screen. Keep at least one visible tab. Removing the selected tab selects the first remaining tab; reordering tabs preserves the selection and page state. Removing a tab unmounts its tab content.

### Initial tab

At startup, if the index page belongs to a tab layout but is omitted from its visible tabs, Ink opens the first visible tab directly. It does not mount the hidden index page or add a redirect to the back stack. Explicit navigation and incoming links to a hidden page still open that page normally. Load saved tab preferences before rendering `Tabs` if they determine the initial selection.

### Tab history

Tab order follows the declarations. A tab group has one place on the back stack. Tab presses, `navigate()` and `replace()` targeting an existing tab group select that tab, preserve its state, and dismiss any pages above the group. They do not create duplicate tab screens. Ordinary pages still create separate instances with `navigate()` and replace the current instance with `replace()`.

### Tab-bar actions

Use `Tabs.Action` for a navigation-bar button that runs an action instead of switching tabs:

```tsx
import { Tabs, navigate } from "ink";
import { wallet, photoCamera, settings } from "ink/icons";

export default function Layout() {
  return <Tabs>
    <Tabs.Screen name="index" icon={wallet} />
    <Tabs.Action icon={photoCamera} onPress={() => navigate("/scan")} />
    <Tabs.Screen name="settings" icon={settings} />
  </Tabs>;
}
```

Here, `app/scan.tsx` sits outside the tab group. Scan opens above the tabs; Back returns to the previously selected tab. Actions follow the JSX order, use the unselected icon colour and never become selected. They do not count towards the requirement for at least one tab or affect the startup selection. Give mapped actions stable React keys.

## Open and close pages

`navigate({ path: "/forecast", params: { placeId } })` or an equivalent `href` opens a screen. Use `back()` to return and `replace()` to replace the current screen. The title-bar button and a left-edge swipe above the keyboard navigate back directly. Hardware Back dismisses the keyboard first. Pass IDs and small JSON values as route parameters.

## Read parameters

`useRouteParams("/album/[id]")` reads named path parameters as strings. The path literal supplies the types without a generated type file; it does not check whether that filename exists. For other parameters, supply a `decode(unknown)` function to validate them. Existing `useRouteParams<T>()` calls remain supported, but a TypeScript generic does not validate runtime data. Unknown or malformed notification routes open “Page unavailable” with a back action.

## Screen state

Covered screens keep their React state. Tabs also keep separate scroll positions. Ink pauses React effects while a screen is hidden and unmounts it when removed from the back stack. Save data in Store if it must survive an app restart.

To save a setting and return, await the store update, then call `back()`. Ink applies the returning screen’s current store snapshot before presenting it, so the previous value doesn't briefly appear.

## External actions

Import `openURL(url)` and `share({ text })` from `ink`. Both return promises and restore the app when the external window closes. These imports add the native `external` capability, including browser package queries. Cancellation resolves normally; invalid URLs, missing handlers and native failures reject. Use [Files and media](/files) to share attachments.

Web links open in Android Custom Tabs. The toolbar follows Ink’s appearance while keeping the site identity and browser security controls visible. If Custom Tabs is unavailable, the command rejects. Telephone links open the dialler; message links open a composer. Other app schemes use their installed handlers. File, content, script and intent URLs are rejected.

