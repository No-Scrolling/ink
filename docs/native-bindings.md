---
title: "Native bindings"
description: "Keep controls mounted and send changed values to Rust."
---

`ink/view` provides an opt-in adapter for screens with a stable structure. Declare controls and their bindings once. JavaScript handles actions and calculates application values; Rust connects those values to the existing native controls. Updates do not render a React component, compare properties or reconcile a JavaScript host tree.

React still mounts and removes the page, so native views can use the existing router alongside ordinary React pages. The Counter example uses this adapter. The template's **Examples → Native bindings** page covers input, navigation, Japanese text, emoji and playback.

## Ownership

| Work | Owner |
| --- | --- |
| Fetching, application decisions, actions and arbitrary derived functions | JavaScript |
| Declarative expressions and retained keyed collections | Rust |
| Page lifetime and navigation | Existing React adapter |
| Persistent controls and binding targets | Rust |
| Keyed row identity, mounted windows and viewport changes | Rust, through compiled `List` rows or `view.list` |
| Layout, drawing, dragging and scrollbars | Existing Rust engine |
| Playback progress and elapsed-time labels | Rust's playback clock |

The declaration is serialised at mount. JavaScript retains application values and action callbacks, plus the serialised declaration needed by React's lifetime handling. It does not retain a mutable tree of native element instances. Rust resolves a changed source to its target properties and uses the same tree and layout engine as the React adapter.

## Declare values

Inside `nativeView(view => ...)`, use `view.value(initial)` for writable application values. Read a value with `.get()` and replace it with `.set(next)`. Objects and arrays must be replaced rather than mutated. Values must be JSON-compatible; functions belong in action properties.

`view.derive([dependencies], calculate)` calculates a read-only value. Dependencies are explicit. Derived values run in declaration order when a dependency changes and can read earlier derived values with `.get()`. `.at(key)` binds an object property or array position; the path must exist in every value supplied.

For arithmetic, comparisons, boolean conditions and label formatting, use `view.compute(expression)` to calculate the value in Rust:

```ts
import { expression, nativeView } from "ink/view";

export default nativeView(view => {
  const count = view.value(0);
  const label = view.compute(expression.concat("Count: ", count));
  return view.node("Screen", {},
    view.node("Text", { text: label, width: 300, maxLines: 1 }),
    view.node("Button", { onPress: () => count.set(count.get() + 1) },
      view.node("Text", { text: "Increase" })),
  );
});
```

Expressions support `add`, `subtract`, `multiply`, `divide`, `remainder`, `less`, `equal`, `not`, `choose`, `concat`, `length` and `pad`. Operands can be literals, bindings or other expressions. Sources must be declared before their dependants. Rust retains results, evaluates only affected expressions and stops propagation when a result is unchanged. Arithmetic requires finite numbers; division by zero rejects. Conditions require booleans. `concat` accepts primitive values, and string lengths count UTF-16 code units. Computed bindings have no JavaScript `.get()`; use `derive` when application code needs a JavaScript function or result.

Fixed-width, single-line Text updates can replace a scene text run without measuring the screen again. Links, pressable text, changing dimensions and other layout changes use the normal layout path.

Pass a value or derived binding directly to a property in `view.node(kind, props, ...children)`. Each child is another node declaration. Button labels are child Text nodes. Actions are ordinary JavaScript functions. Declare nodes, values, derived values and mount hooks inside the build callback; declaring them after construction throws.

Synchronous value changes are batched into one update. Replacing a value with an identical reference or primitive does nothing. Native property validation still applies, including boolean visibility. Bound text updates must be strings.

## Manage a view's lifetime

`view.onMount(start)` runs when a view becomes active. Return a function to unsubscribe or dispose of resources. React Activity hiding disconnects these hooks and action handlers. Changes made while hidden are retained and applied in the reveal commit before the updated scene is presented. Mount-time changes are included with the creation commit too.

Removal disconnects callbacks and cancels pending updates. A later fresh mount constructs new state. This adapter does not read changing React props; use application values and mount-time subscriptions for external state.

For controlled TextInput nodes, update both `value` and `eventCount` from the native `onChange(value, eventCount)` event. This preserves the existing stale-edit protection.

## Keyed lists

`view.list(items, { key: "id", gap: 12 }, item => ...)` declares a row template once. `items` is a binding to an array of objects with unique string keys. Use `item.at("field")` in the template. Rust compares keyed data, materialises the visible window with overscan, and retains the visible anchor through insertion and reordering. Content dragging, thumb dragging and window changes do not call JavaScript.

```ts
const items = view.collection([{ id: "one", title: "First episode" }], "id");
const list = view.list(items, { key: "id" }, item =>
  view.node("Text", {
    text: item.at("title"),
    onPress: key => openEpisode(key),
  }),
);
```

`view.collection(items, key)` retains keyed records in Rust. Use `insert(item, beforeKey?)`, `update(key, fields)`, `remove(key)`, `move(key, beforeKey?)`, `reverse()` and `reset(items)`. Omitting `beforeKey` appends the item. Keys are unique strings and cannot change through `update`. Field patches merge into a record, including explicit `null` values. Synchronous edits are batched; editing a field sends that field rather than the dataset. Reordering still takes work proportional to the key order. Collections have no JavaScript `.get()`; keep app-specific state separately or use SQL to select records.

Bind `expression.length(items)` for a native collection count. A controlled row input can call `items.update(key, { text, eventCount })` directly. `db.queryInto(items, sql, parameters, { signal })` replaces a collection without sending query rows through JavaScript. Results older than an edit or result already applied in Rust are ignored. Keep the existing `view.value(array)` path when application code needs to read or replace an array in JavaScript.

Row callbacks receive the key before their usual arguments. A row input receives `onChange(key, text, eventCount)`; store the text and acknowledgement count in that keyed item's data. Rows outside the window are removed, so durable row state belongs in the application data. Native edit acknowledgements remain monotonic when a row is remounted.

Options include `initialEnd`, `followEnd`, `onStartReached` with `hasOlder`, and `onEndReached` with `hasMore`. Boundary callbacks request application data once per changed boundary; they do not expose window indices or ask JavaScript to mount rows. A newly mounted empty list requests each enabled boundary once. Further requests need a changed boundary. Use `hasOlder` and `hasMore` to disable requests while loading or when no more data is available.

Templates bind row fields or literals. Put derived row values in the item data; external bindings, nested lists and changing the template after mount are not supported. The public React `List` automatically uses this engine for supported rows and keeps React rendering for arbitrary `renderItem` functions.

## Existing React lists

Keep using `List` from `ink`. Rebuild the app with the updated SDK; there is no new prop or migration.

The compiler converts inline, fixed `Text`, `Stack`, `Row`, `Image`, `Icon`, `Avatar`, `Toggle`, `Button` and `Field` declarations into native row templates. It recognises imported aliases and preserves local shadowing. JavaScript projects item values and keeps application callbacks. Rust owns row creation, recycling, keyed identity and viewport changes. Scrolling these lists does not ask React to render a new window.

Custom components, hooks, calls in rendered values, conditional row structures, spreads and refs keep the existing React implementation automatically. React content passed indirectly to a text child also falls back. This preserves arbitrary React behaviour; it does not make every possible `renderItem` native. A change between template structures or between native and React rendering remounts the list.

Compiling a row does not remove all JavaScript work. Changed item arrays or captured visual dependencies project the dataset again and transfer changed records to Rust. Each row projection evaluates its expressions once; there is no separate dependency evaluation. For compiler-proven field key extractors, a fresh action closure alone skips that scan while keeping callback state fresh. This favours scrolling predictable rows; it is not a guarantee that initial mounting or replacing a large dataset is faster.

The native path expects immutable plain records and arrays. Replace changed records and their containing array. Accessors, proxies, custom prototypes and cyclic data keep React rendering; eligibility checks detect proxies in the runtime and inspect descriptors without invoking getters. Use a block-bodied `renderItem` for observable coercion. Captured bindings are read when projecting a non-empty list, and guarded property accesses stay inside the row expression.

The native path preserves the public pagination callbacks and retry UI, `initialEnd`, `followEnd`, `gap` and `measurementKey`. The `view.list` API remains available within explicitly declared native views.

## Native capabilities

Literal declarations such as `view.node("TextInput", ...)` and `view.node("Image", ...)` declare keyboard and image requirements during bundling. Modules removed by bundling do not add release capabilities. If a helper chooses its node kind dynamically, declare the corresponding `text-input`, `image` and `network` capabilities in `ink.toml`. Import icon references from `ink/icons` as usual so the compiler includes their assets.

## Native composites

`Row` and `Avatar` each supply one native host description. Rust constructs their image, icon and text children, retaining the existing geometry and press behaviour. JavaScript handles navigation and app callbacks. Arbitrary React children still use React.

## Player construction and playback labels

The existing React `PlayingScreen` API now supplies one native player description. Rust constructs the artwork, labels, progress, transport and action controls. Existing applications receive this change without rewriting their screens. Rust advances the elapsed label from the same clock as the bar, changing the string once per second. JavaScript supplies authoritative position and duration in milliseconds, playing state and meaningful actions. Pause and seek still require application state updates; the native clock does not control audio playback.

## Current scope

This adapter supports persistent control structures, bound properties and keyed lists. Applications opt into `nativeView`; supported ordinary `List` rows compile automatically. Other React screen components still reconcile in JavaScript. A count change in the 500-cell binding benchmark hides predeclared cells; the separate 1,000-row native-controls fixture exercises keyed insertion, reordering, removal and window recycling.

The 500-cell timed workload sends one tick; native expressions update every label and all 50 row gaps. Compare it with the unchanged React workload using [the headless harness](../benchmarks/headless/README.md). Both still exercise the same native layout and scrolling code.
