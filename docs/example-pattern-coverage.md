# Example pattern coverage — 6 September 2026

Compared routes and shared components in Buses, Index, Spotify, Beeper, Reverb, Weather and Passes on 6 September 2026. The table records Ink’s examples, not completed app ports. Provider and native integrations still need real-app checks.

| App | Implemented examples and shared behaviour | Remaining real-app validation |
| --- | --- | --- |
| Buses | Lists, live values, settings, title-bar actions and reordering with up/down controls. | Departures data, multi-column layouts and favourite persistence. |
| Index | Wrapping title/subtitle rows, detail text, loading/error states and selection. | Feed fetching, metadata mapping and full detail workflows. |
| Spotify | Artwork rows, automatic pagination, title-bar actions and PlayingScreen with progress, seeking and transport controls. | Album detail composition, authentication, queue and actual playback. |
| Beeper | Single/group conversations, text and images, replies, status/retry, reactions, message actions, older-history loading and a multiline composer. | Real history, incoming messages, sending failures, attachments and service integration. Video messages are not implemented. |
| Reverb | Artwork/title/subtitle rows, search and PlayingScreen with optional image, title/artist actions, long-press seeking and configurable bottom actions. | Full album/detail flow and playback integration. |
| Weather | Search, fields, selection, title-bar actions and reordering. | Input above saved locations, forecast columns and data/persistence. These remain app compositions rather than dedicated Ink components. |
| Passes | Scanning API examples, reordering and a Code generation page covering all 13 supported formats. | Naming, saving, renaming, scanning and reopening real passes as one workflow. |

## Shared component status

`Row`, `Screen.rightAction`, automatic `List.onLoadMore`/`hasMore`, PlayingScreen and ConversationScreen are implemented. Reordering uses ordinary React state and buttons; there is no specialised reorder API.

ConversationScreen takes message data and callbacks. It owns rendering, the actions page, reply previews, keyboard accommodation and scroll behaviour. Its composer grows to three lines, scrolls longer drafts and uses Return for newlines. Sending dismisses the keyboard and requests the bottom position with the message update. History insertion preserves the reader's anchor during a drag.

The player and conversations use local fixtures. Playback, messaging and attachment services remain app responsibilities. General-purpose arbitrary pressable content, a public bottom-composer slot and video are not implied by these specialised components.

See [the component guide](ink.md), [public-prop audit](public-props-audit.md) and [verification record](verification-2026-09-06.md). The next agreed step is joint real-app integration on the LP3, followed by fixes for demonstrated gaps rather than additional speculative components.
