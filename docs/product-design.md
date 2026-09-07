---
title: "Product design"
description: "Ink's authoring model, implemented scope and remaining integration work."
---

Ink is a React and TypeScript framework for Light Phone III apps. Developers supply app data and behaviour; Ink supplies the phone's visual language, native interaction and device integration.

Module guides describe current APIs.

## Authoring

Functions, hooks, context, composition, conditional rendering and array mapping use ordinary React semantics. Native requirements come from imported modules, not special rules about application expressions. Use standard networking and compatible JavaScript libraries.

Ink does not promise React Native native-module, DOM or Node compatibility. Its runtime facilities are listed in [runtime compatibility](/runtime-compatibility).

## Screens and behaviour

`Screen` provides navigation, spacing, scrolling and keyboard handling. `Row`, `PlayingScreen` and `ConversationScreen` provide common layouts. Your app supplies data, provider requests, playback and message delivery.

Compose other layouts with `Text`, `Image`, `Button`, `Stack` and `List`. Lists measure rows and trigger pagination automatically. ConversationScreen also handles replies, message actions and keyboard scrolling.

Do not turn each app layout into a framework feature. Weather columns, album detail composition and saved-location screens can use existing primitives.

## Native modules

Commands return promises. Continuing work has an explicit owner; React hooks manage screen-owned work. Observations use the existing loading/ready/error snapshot shape. Persistent operations such as downloads survive observers disappearing.

Small APIs stay small: Clipboard copies text, Secure Store saves secrets, and Location separates a single fix from tracking. Background jobs receive saved input rather than capturing UI state.

Include capabilities through explicit module imports. Optional engines and capture features must not increase every app's native footprint. Reuse native engines behind Ink interfaces; provider-specific behaviour stays in provider integrations.

## Implemented module scope

| Capability | Current scope |
| --- | --- |
| [Auth](/auth) | Browser and device-code sign-in, session observation, coordinated refresh and local sign-out. |
| [Connectivity](/connectivity) | Observe network state; request success remains an HTTP concern. |
| [Downloads](/downloads) | Durable ordinary HTTP files; provider download engines remain separate. |
| External actions | Web links and browser sign-in use Android Custom Tabs with explicit unavailable errors when unsupported. Phone, mail and other supported external URLs dispatch to installed handlers. Text sharing uses a framework command; file sharing belongs with files and media. |
| [Files and media](/files) | An Ink photo/video gallery with multi-selection, document picker actions, shared durable file references, image preparation, saving and sharing. |
| [Maps](/maps) | An optional MapLibre view filling the screen's content area, with markers, native gestures and explicit camera movement. |
| [Secure Store](/secure-store) | Small encrypted string values, shared by Auth and app code. |
| [Store](/store) | JSON storage and direct read-only SQLite queries over imported bundled databases. |

Reader, Bluetooth and Sensors are outside scope. There is no separate Records or System module. Map clustering, offline map regions, general image editing and video playback are not part of this scope.

## Shared files and optional engines

Picking, capture, recording and downloads return a shared `FileRef`. Use its source for display or upload, and save its ID to reopen it later. `fetch(file.src).blob()` keeps file data native through slicing and multipart upload. Text and byte reads copy the data into JavaScript.

Completed recordings are retained independently until explicitly deleted. Apps persist file IDs and own reference counting and retention; disposing a screen does not delete accepted attachments.

Audio capture is imported from `@ink/audio/capture`; camera scanning is imported from `@ink/barcode/scan`. Playback and still-photo imports exclude those optional native engines. MapLibre and other optional engines follow their module capability graph.
