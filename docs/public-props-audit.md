# Public component prop audit

This source audit compares the public exports in `packages/ink/src/index.ts`, `navigation.ts`, `patterns.ts`, `list.ts`, `playing.ts` and `conversation.ts` with native parsing, layout and interaction handlers. It includes the keyboard/composer follow-up. Source coverage does not establish every physical-device, delayed-input or lifecycle scenario; focused interaction checks are recorded separately in [verification](verification-2026-09-06.md).

| Public surface | Implementation path and scope |
| --- | --- |
| `Screen.children`, `header`, `title`, `centered`, `rightAction` | Native screen children, title and vertical content distribution; overflowing body content scrolls while the optional composed header stays pinned. The right action reserves title space and forwards its icon and press callback. |
| `Stack.children`, `axis`, `gap`, `align`, `justify` | Native horizontal/vertical measurement, cross-axis placement and main-axis distribution. Stretch applies to stretchable controls/text/stacks; an image retains its explicit dimensions. |
| `Text.children`, `size`, `align`, `maxLines` | Native text flattening, font sizing, alignment, line breaking and final-line ellipsis. Nested Text does not introduce independently styled spans. |
| `Button.children`, `onPress`, `href`, `disabled`, `selected`, `icon` | Text/icon rendering and native hit regions. JavaScript translates href to navigation and rejects combining it with onPress. Disabled removes its action; selected controls underlining. |
| `Row.title`, `subtitle`, `image`, `href`, `onPress` | Native whole-row hit region, wrapping text and reserved artwork geometry. Uses normal Image loading and navigation. |
| `Field.label`, `children`, `onPress`, `href` | Native label and wrapping text value; the whole field gets the optional action. Children cannot be layout controls or inputs. |
| `Toggle.label`, `value`, `disabled`, `onChange` | Native toggle state/mask and an event carrying the inverse value. Disabled removes its action. |
| `Icon.name`, `size`, `tone` | Imported reference resolves a bundled raster mask; native dimensions scale it and tone selects primary/muted colour. |
| `Image.src`, `width`, `height`, `fit`, `bleed`, `zoomable` | Imported assets, HTTPS and camera-managed source selection; explicit positive dimensions, fit transform, full-width layout, optional native pinch/double-tap/pan handlers. Arbitrary filesystem URLs and parent-only sizing are unsupported. |
| `TextInput.value`, `onChange`, `onSubmit`, `placeholder`, `autoFocus`, `action` | Native editing, cursor placement, event counters and muted placeholders. Search/Done submit; Return inserts newlines and grows to three lines with internal vertical scrolling. Editing preserves scroll while the cursor is visible. The keyboard has a bottom inset and no dismiss row. Delayed controlled updates still need broader verification. |
| `List.items`, `keyExtractor`, `renderItem`, `gap`, `followEnd`, `measurementKey`, `onLoadMore`, `hasMore`, `onLoadOlder`, `hasOlder`, `initialEnd` | Keyed windows, measured heights and bounded metadata. Native overscan triggers sequential pagination in either direction; failures expose retry. Initial-end mounting supports conversations. Anchors preserve position across prepends and active drags; end-following pauses during touch. |
| `PlayingScreen` | Optional artwork, title/artist callbacks, controlled progress and transport, long-press actions and configurable bottom controls. App owns actual playback. |
| `ConversationScreen` | Message data, controlled draft and send callback; optional group authors, history, attachment, retry, image, double-tap and custom-action callbacks. Owns rendering, reply/actions navigation, three-line composer, keyboard dismissal and explicit bottom scrolling on send. Loading/sending flags control presentation and interaction. |
| `Message` | Standalone text/image, timestamp, outgoing alignment, author, reply, reactions and delivery status; optional retry, long-press, double-tap and image/reply press handlers. |
| `Navigator`, `Route.path`, route children and route parameters | JavaScript validates routes, retains covered screens through Activity and supplies native Back actions. External unknown paths render the unavailable screen; decoding parameters remains the app's responsibility. |
| `Tabs`, `Tab.id`, `icon`, `children` | Stable tab identity, current selection and Activity visibility in JavaScript; native icon bar and selection hit regions. |
| `SettingsChoices` | Each option becomes a selected/disabled Button with the supplied value callback. |
| `LoadingState`, `EmptyState`, `ErrorState` | Text/Stack/Button composition; labels, descriptions, actions, retry and disabled states are forwarded. |
| `Confirmation` | Native screen and fixed footer action; wrapper selects and uppercases the pending/normal label. Pending removes the footer action. Children are message text. |

This audit corrected the earlier claims about image sources, sizing and fixed-height lists. It also recorded nested-text and image-stretch limits. Managed file support was added later; see [Files](files.md) for the current API.

The list review fixed measurement-cache eviction, unnecessary invalidation after prepends and loss of visible row identity. Ink now keeps both scrollbar widths, versions content per key and follows the existing first key when data changes.

Rust and TypeScript compilation passed. No tests or device checks were added in this audit; it does not verify every combination of props or lifecycle timing.