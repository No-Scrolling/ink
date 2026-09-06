# Example pattern coverage — 6 September 2026

Source comparison of the route trees and representative screen/shared-component implementations in `~/Developer/buses`, `index`, `spotify`, `beeper`, `reverb`, `weather` and `passes`. This is a UI-pattern audit, not a device walkthrough, exhaustive route verification or confirmation that the apps' native dependencies work in Ink. No app was changed or ported.

## Existing coverage

Ink demonstrates tabs and navigation, dedicated search input and results, settings toggles and selection, wrapping text, fields, images, dynamic content, virtualised lists, following appended items, fixed top controls, loading/error/retry and bottom-action confirmation. Modules separately demonstrate camera scanning, photography, audio commands, networking and other device APIs.

| App | Representative source | Coverage and gaps |
| --- | --- | --- |
| Buses | `app/bus-stop/[atcoCode].tsx`, `components/ReorderItem.tsx` | Lists, live values and settings have basic equivalents. Multi-column departures, favourite title-bar actions and reordering lack equivalent examples. |
| Index | `app/(tabs)/index.tsx`, `app/feed/[id].tsx`, `components/OptionsSelector.tsx` | Loading, empty content, detail text and selection have equivalents. Whole-row interaction with primary text and secondary date metadata is not demonstrated. |
| Spotify | `shared/components/DetailScreen.tsx`, `app/playing.tsx`, `app/album/[id].tsx` | Images, audio commands, lists and confirmation are covered individually. Artwork plus tracks, pagination, title-bar actions and an interactive progress bar are not covered as complete patterns. |
| Beeper | `app/chat/[roomId].tsx`, `components/MessageInput.tsx` | Following, text wrapping and images are covered individually. Mixed message rows, older-history loading, a keyboard-aware bottom composer, long-press/double-tap actions and video are not equivalent to the current examples. |
| Reverb | `components/MediaListItem.tsx`, `app/playing.tsx`, `app/album/[id].tsx` | Search and standard controls are represented. Whole-row artwork/title/subtitle interaction and the complete player/detail layouts remain gaps. |
| Weather | `app/(tabs)/search.tsx`, `app/search/weather.tsx`, `app/settings/reorder-details.tsx` | Search, units, fields and selection are represented. Input above saved locations, forecast columns, favourite title-bar actions and reordering are not demonstrated together. |
| Passes | `app/reorder-passes.tsx`, `app/detail/qrDisplay.tsx` | Scanning, images, input and confirmation have examples. Reordering and barcode generation/display with naming, saving and renaming remain uncovered workflows. |

## Most useful additions

1. A rich list/detail example: rows with primary/secondary text and optional artwork, opening a detail view with an image and content. Include wrapping or expanding content so automatic height measurement is exercised again. The simplified Follow new items page now contains only uniform text rows.
2. Reordering with up/down actions, as used by Buses, Passes and Weather. This can start as composition; the audit does not justify a dedicated native reorder component.
3. A conversation fixture combining history insertion, images, following and keyboard-aware input. Keep it local until the agreed joint real-app session.
4. An album/player fixture combining artwork, track rows, loading more and progress interaction. Audio module commands alone do not establish this layout.

Title-bar actions deserve a shared design decision because they recur across the apps. Weather's input-plus-saved-content pattern can extend an input example without changing the intentionally minimal Search tab.

## Framework seams exposed

Some gaps need more than sample code. Current Button and Field children are rendered as text, so arbitrary tappable artwork/text compositions are not supported through those components. Screen exposes a title and composed fixed header, but not a title-bar action or general bottom composer slot. List has no public reach-start/reach-end notification for automatic pagination. The public controls do not expose general long-press/double-tap handlers or a seek-bar component.

These are candidates to evaluate against real layouts, not a request to add every React Native prop. Prefer composition where it works and introduce only the shared interaction or layout capability that cannot otherwise be expressed. Existing framework implementation is delivered, but these broader app patterns are not all covered or verified.

## Follow-up implementation

The template now includes Rows (optional artwork, wrapping title/subtitle and whole-row navigation), the Examples title-bar action opening Action Page, automatic near-bottom Pagination, and a centred Barcode encoding `Hello World!`. The public additions are `Row`, `Screen.rightAction` and `List.onLoadMore`/`hasMore`. This closes those specific example/API gaps; it does not establish complete album, barcode-saving or chat workflows. General bottom composers, video, reorder examples and player progress remain outside these additions.

The Reordering example now demonstrates eight keyed rows with up/down icon buttons and disabled boundary actions. It uses normal React array updates and screen composition; no specialised reorder API was added.
