# Design apps with Ink

Build Light Phone III apps from Ink's screens and components. Ink already supplies typography, spacing, padding, colours, navigation, scrolling and keyboard behaviour. Your job is to choose the right page for each task and the right component for each piece of content.

Follow the steps in order. Do not write screen code until you have a page map.

## 1. Write a page map

List every page before you build. Give each one a route, a single page type from [Page types](#2-choose-a-page-type) and the page it is reached from.

```text
/                     Main task      Tuner reading. Header: settings icon → /settings
/settings             Settings       Toggles, Reference pitch field → /reference
/reference            Choice         Reference pitch options. Saves and goes back
```

Shape the map with these rules:

- `/` opens on the app's main task. Do not make it a menu of buttons that lead to the real content. Add setup or welcome pages only when the app cannot work without them, and leave them once setup is complete.
- Use tabs only for two to four peer sections that users switch between often, such as Today, Locations and Settings. Put tab pages in `app/(tabs)/`.
- Reach Settings from a header icon or a tab. Reach per-item actions from a `moreHoriz` header icon or a long press, which opens an options page.
- Keep the tree shallow: section → item → options or choice. If a page needs a fourth level, group the settings differently.
- When a page has two jobs, it is two pages. Split it.

## 2. Choose a page type

Every page is exactly one of these types. Use the listed components and nothing else at the top level of the page.

| Page type | Use it for | Build with |
| --- | --- | --- |
| Main task | The app's primary reading or tool | `Screen` with the one thing users came for. Use `centered` for short content. Header icons for Settings and secondary pages |
| Collection | Items users browse or open | `List` of `Row`. `EmptyState` when there are no items. `add` header icon to create |
| Detail | One item | `Screen` titled with the item name. Content first, `moreHoriz` header icon for actions |
| Settings | Preferences | `Toggle`, `Field` and `Button` as direct children of `Screen` |
| Choice | Pick one value for a setting | `SettingsChoices` only. Save, then `back()` |
| Input | Enter or edit one value | One `TextInput` with `autoFocus`. Its keyboard action submits. Hints or errors underneath only |
| Options | Actions for one item | `Button`s only, one per action |
| Confirmation | Destructive or irreversible action | `Confirmation` as the whole page |
| Media or chat | Playback or messages | `PlayingScreen` or `ConversationScreen` |
| Custom visual | Diagrams, boards, gauges | `Canvas` primitives inside `Screen`, with Ink components for any controls |

Loading, empty and error states are not separate pages. Place `LoadingState`, `EmptyState` or `ErrorState` directly inside `Screen` in place of its content.

## 3. Choose components

Pick the component that matches what the element means, not how you want it to look.

| To show | Use | Not |
| --- | --- | --- |
| A setting that is on or off | `Toggle label="…"` | A button whose label changes |
| A setting with a current value | `Field label="…" href="/choice"` showing the value | A button labelled with the value |
| A group of related settings | `Button href="/settings/group"` labelled with the group name | `Field` with an empty value |
| An action | `Button` labelled with a verb | `Text` with `onPress` |
| An item that opens | `Row` with `title`, and `subtitle` or `image` when useful | A `Stack` of `Text` with `onPress` |
| A scrolling collection | `List` | `.map()` over many items inside `Stack` |
| A user-ordered collection | `ReorderList` on its own page, reached from a `swapVert` header icon | Up and down buttons |
| A one-of-many choice | `SettingsChoices` on a choice page | Toggles that turn each other off, or `+` and `−` buttons |
| Content that stays above scrolling | `Screen header` | A custom pinned view |
| Up to three persistent bottom actions | `Screen actionBar` | Buttons placed against the screen edge |
| A secondary navigation action | `leftAction` or `rightAction` with an icon from `ink/icons` | A button in the content |
| Supporting text | `Text size={18}` below what it explains | A paragraph above the controls |
| A large primary reading | One `Text` with a large `size` and `tabularNumbers` | Several large sizes on one page |

Common header icons: `settings` opens Settings, `add` creates an item, `moreHoriz` opens options, `swapVert` reorders, `sort` changes order, `check` saves an edit, and `graphicEq` opens the playing screen.

## 4. Lay out the page

- Put components directly inside `Screen`. `Screen` already spaces its children 47 pixels apart and supplies side padding and scrolling. Do not wrap the whole page in a `Stack`.
- Use `Stack` only for elements that belong together and need tighter spacing than the page, such as a reading above its unit, or a horizontal row of labels. Use a gap of 16 or less inside a group.
- Use `Stack justify="space-between"` as the only child of `Screen` to pin a footer to the bottom.
- Order a page by importance: primary content, then the controls that act on it, then supporting text.
- Order a Settings page as values first (toggles and fields), then links to groups, then destructive actions.
- Use default text size for content and labels and `size={18}` for supporting text. Add at most one larger size per page, for its primary reading.
- Centre short content with `Screen centered`. Align feeds, lists and forms to the top.

## 5. Keep it small

Ink apps are deliberately plain. Remove anything that does not help the user complete the task.

- Do not create wrapper components that restyle or rearrange Ink components, such as cards, sections, headers, tiles or custom list items. A local component is fine when it composes Ink components for reuse across pages without changing their appearance.
- Do not add borders, backgrounds, dividers, badges, status pills or decorative icons.
- Do not set colours outside `Canvas`. Inside `Canvas`, derive colours from `useColourScheme()`.
- Do not use props that are not in Ink's type definitions. If TypeScript rejects a prop, change the design, not the types.
- Do not add explanatory paragraphs to a page. One short sentence is acceptable when the purpose or limits of a control are not obvious.
- Do not add success messages for actions whose result is visible. Navigate back, or show the changed value.
- Do not add a toggle subtitle that repeats its label. Add one only when the label alone cannot be understood.
- Do not show the same information twice, such as a status in both a label and a sentence.

## 6. Handle state and input

- Inputs live on their own page. Use the right keyboard; its action submits or saves. Do not add a Save or Cancel button. The header Back button cancels.
- Search submits to a separate results page. Multiline inputs keep Return for new lines and use a `check` header action to save.
- Keep existing content visible during refreshes and when returning to a page. Show `LoadingState` only when there is no content yet.
- Keep a control's label and appearance while its action is pending. `useAction` already ignores repeated calls, so do not set `disabled` or change the label to “Loading…”. Reserve `disabled` for controls that are unavailable.
- Show one error at a time. Use `ErrorState` with a retry action when the page cannot show its content. Otherwise, place one `Text` beside the affected control.
- Request a permission when the user opens the feature that needs it. Do not add a separate permission button.
- Preserve scroll position, drafts and selections when navigating.
- Use `tabularNumbers` for changing numbers. Reserve space for images whose size is known.

## Example: a setup page

This keyboard setup page mixes statuses, actions, instructions and several error messages:

```tsx
<Screen title="Set up keyboard">
  <Stack gap={24}>
    <Button onPress={enable.run}>{ready?.enabled ? "Atlas is enabled" : "Enable Atlas"}</Button>
    <Button onPress={choose.run}>{ready?.selected ? "Atlas is selected" : "Choose Atlas"}</Button>
    <Button onPress={scroll.run}>Open Scroll assistant</Button>
    <Text size={18}>Select Sliding Mode 1 for each app where you want to scroll and swipe type. Atlas reads the keyboard while a text field is active, and scrolling works everywhere else.</Text>
    {(enable.status === "error" || choose.status === "error") && <Text>Could not open keyboard settings.</Text>}
    {scroll.status === "error" && <Text>Scroll assistant is only available on the Titan 2.</Text>}
    {/* Retry button with disabled and pending label, two more error messages */}
  </Stack>
</Screen>
```

Problems:

- Button labels change between an action and a status.
- The whole page is wrapped in a `Stack` with its own gap.
- A two-sentence paragraph sits between controls.
- Each action adds its own error line at the bottom of the page.

The same page as a Settings page type:

```tsx
<Screen title="Set up keyboard">
  {loaded.status === "error" ? <ErrorState message="Could not read keyboard settings." onRetry={reload} /> : <>
    <Field label="Enabled" onPress={enable.run}>{ready.enabled ? "Yes" : "No"}</Field>
    <Field label="Selected" onPress={choose.run}>{ready.selected ? "Yes" : "No"}</Field>
    <Field label="Scroll assistant" onPress={scroll.run}>Sliding Mode 1</Field>
    {error && <Text size={18}>{error}</Text>}
  </>}
</Screen>
```

Each row shows one status and opens the place to change it. Spacing comes from `Screen`, and there is one error line.

## Before you finish

Check each page against this list and fix every failure before reporting the work as done:

- [ ] The page appears in the page map with one page type.
- [ ] `/` opens on the main task.
- [ ] Top-level children of `Screen` are the components listed for its page type.
- [ ] No wrapper `Stack`, custom gaps between page sections, borders, colours or decorative elements.
- [ ] No component restyles or reimplements an Ink component.
- [ ] Labels name actions or settings; values and statuses are in `Field` values or `Toggle` states.
- [ ] No paragraphs, repeated information or success messages.
- [ ] One error at a time, beside the control or as `ErrorState`.
- [ ] `ink check` passes. Its `ink-design` rules catch some of these mistakes; fix the design rather than disabling a rule.
- [ ] Each page is checked in the emulator with the keyboard open, long content, missing data, light and dark modes, and after navigating back from another page. Use the phone for hardware features.

Use plain British English and sentence case. Name buttons with direct verbs, such as “Export words”.

## Reference apps

These apps show the intended structure. Read the matching page in one before building a new page type.

| App | Shows |
| --- | --- |
| [Tuner](https://github.com/No-Scrolling/tuner) | Main task opening on a reading, Settings with a choice page |
| [Weather](https://github.com/No-Scrolling/weather) | Tabs, grouped Settings, unit choice pages, search input and results |
| [Podcasts](https://github.com/No-Scrolling/podcasts) | Collection, detail, options page, Player settings, `PlayingScreen` |
| [Passes](https://github.com/No-Scrolling/passes) | Collection with reorder, options page, rename input, delete confirmation |

See the [Ink documentation](https://ink.noscroll.ing/screens) for each component's props and examples. Ink's type definitions in `node_modules/ink/src` list every supported prop.
