# Public component prop audit

This source audit compares the public exports in `packages/ink/src/index.ts`, `navigation.ts`, `patterns.ts` and `list.ts` with `crates/ink-core/src/react.rs`, native layout and native interaction handlers. It establishes that the documented props have implementation paths; it does not establish physical-device behaviour, delayed-input correctness or visual quality.

| Public surface | Implementation path and scope |
| --- | --- |
| `Screen.children`, `title`, `centered` | Native screen children, header and vertical content distribution; overflowing content scrolls. |
| `Stack.children`, `axis`, `gap`, `align`, `justify` | Native horizontal/vertical measurement, cross-axis placement and main-axis distribution. Stretch applies to stretchable controls/text/stacks; an image retains its explicit dimensions. |
| `Text.children`, `size`, `align`, `maxLines` | Native text flattening, font sizing, alignment, line breaking and final-line ellipsis. Nested Text does not introduce independently styled spans. |
| `Button.children`, `onPress`, `href`, `disabled`, `selected`, `icon` | Text/icon rendering and native hit regions. JavaScript translates href to navigation and rejects combining it with onPress. Disabled removes its action; selected controls underlining. |
| `Field.label`, `children`, `onPress`, `href` | Native label and wrapping text value; the whole field gets the optional action. Children cannot be layout controls or inputs. |
| `Toggle.label`, `value`, `disabled`, `onChange` | Native toggle state/mask and an event carrying the inverse value. Disabled removes its action. |
| `Icon.name`, `size`, `tone` | Imported reference resolves a bundled raster mask; native dimensions scale it and tone selects primary/muted colour. |
| `Image.src`, `width`, `height`, `fit`, `bleed`, `zoomable` | Imported assets, HTTPS and camera-managed source selection; explicit positive dimensions, fit transform, full-width layout, optional native pinch/double-tap/pan handlers. Arbitrary filesystem URLs and parent-only sizing are unsupported. |
| `TextInput.value`, `onChange`, `onSubmit`, `placeholder`, `autoFocus`, `action` | Native input state, event counters, keyboard action, placeholder and focus. A submit handler receives native text and retains focus; without one submit dismisses. Delayed controlled updates and composition still require interaction verification. |
| `List.items`, `keyExtractor`, `renderItem`, sizing, `gap`, `followEnd`, `measurementKey` | JavaScript mounts keyed rows and sends bounded key/content metadata; native measurement, offset lookup, window events and anchor correction use it. Fixed sizing skips measurement. Changing measurementKey invalidates cached geometry derived from external state. |
| `Navigator`, `Route.path`, route children and route parameters | JavaScript validates routes, retains covered screens through Activity and supplies native Back actions. External unknown paths render the unavailable screen; decoding parameters remains the app's responsibility. |
| `Tabs`, `Tab.id`, `icon`, `children` | Stable tab identity, current selection and Activity visibility in JavaScript; native icon bar and selection hit regions. |
| `SettingsChoices` | Each option becomes a selected/disabled Button with the supplied value callback. |
| `LoadingState`, `EmptyState`, `ErrorState` | Text/Stack/Button composition; labels, descriptions, actions, retry and disabled states are forwarded. |
| `Confirmation` | Native screen and fixed footer action; wrapper selects and uppercases the pending/normal label. Pending removes the footer action. Children are message text. |

The audit corrected stale documentation that claimed arbitrary managed-file images, parent-only image sizing and fixed-height-only lists. It also records the nested-text and image-stretch limitations instead of implying CSS-like layout or inline styling.

A second review of list bookkeeping corrected three code issues: full-width/scrollbar-width passes evicted each other's measurements; prepending invalidated unchanged rows' cached heights; and retaining a numeric JavaScript window could unmount visible row identities during prepends. The implementation now retains two width variants, versions content per key, and maps the existing window's first key before rendering changed data. Native anchors are chosen deterministically when more than one list is visible.

Rust and TypeScript compilation are the bounded checks for these edits. No automated tests or device operations were added for this audit. Source coverage is not a claim that every combination of props, layout nesting or lifecycle timing has been exercised.
