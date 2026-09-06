---
title: "Product design"
description: "Ink's authoring model, implemented scope and remaining integration work."
---

Ink is a React and TypeScript framework for Light Phone III apps. Developers supply app data and behaviour; Ink supplies the phone's visual language, native interaction and device integration.

This is the product design reference. Module guides describe the current interfaces and their limits. Historical plans and verification records describe work at the time, not additional product requirements. Implemented interfaces and verified provider integrations are separate claims.

## Authoring

Functions, hooks, context, composition, conditional rendering and array mapping use ordinary React semantics. Native requirements come from imported modules, not special rules about application expressions. Use standard networking and compatible JavaScript libraries.

Ink does not promise React Native native-module, DOM or Node compatibility. Its runtime facilities are listed in [runtime compatibility](runtime-compatibility.md).

## Screens and behaviour

Screen owns navigation chrome, spacing, scrolling and keyboard accommodation. Row, PlayingScreen and ConversationScreen hide shared layout and interaction behind data and callbacks. Apps own provider requests, provider-specific playback engines, message delivery and persistence policies.

Keep ordinary composition available through Text, Image, Button, Stack and List. Every List is virtualised and measures its content automatically. Ink triggers pagination; the app fetches and appends or prepends data. ConversationScreen additionally owns message actions, reply selection and keyboard/scroll coordination.

Do not turn each app layout into a framework feature. Weather columns, album detail composition and saved-location screens can use existing primitives.

## Native modules

Commands return promises. Continuing work has an explicit owner; React hooks manage screen-owned work. Observations use the existing loading/ready/error snapshot shape. Persistent operations such as downloads survive observers disappearing.

Keep the simple operation simple. Clipboard and Secure Store expose a few commands. Location separates a one-off fix from ongoing tracking. Background jobs use durable input rather than captured foreground state.

Include capabilities through explicit module imports. Optional engines and capture features must not increase every app's native footprint. Reuse native engines behind Ink interfaces; provider-specific behaviour stays in provider integrations.

## Implemented module scope

| Capability | Current scope |
| --- | --- |
| [Auth](auth.md) | Browser and device-code sign-in, session observation, coordinated refresh and local sign-out. |
| [Connectivity](connectivity.md) | Observe network state; request success remains an HTTP concern. |
| [Downloads](downloads.md) | Durable ordinary HTTP files; provider download engines remain separate. |
| External actions | Web links and browser sign-in use Android Custom Tabs with explicit unavailable errors when unsupported. Phone, mail and other supported external URLs dispatch to installed handlers. Text sharing uses a framework command; file sharing belongs with files and media. |
| [Files and media](files.md) | An Ink photo/video gallery with multi-selection, document picker actions, shared durable file references, image preparation, saving and sharing. |
| [Maps](maps.md) | An optional MapLibre view filling the screen's content area, with markers, native gestures and explicit camera movement. |
| [Secure Store](secure-store.md) | Small encrypted string values, shared by Auth and app code. |
| [Store](store.md) | JSON storage and direct read-only SQLite queries over imported bundled databases. |

Reader, Bluetooth and Sensors are outside scope. There is no separate Records or System module. Map clustering, offline map regions, general image editing and video playback are not part of this scope.

## Shared files and optional engines

The existing screen components remain the intended design. Capture, picking, recording and downloads now produce the same managed `FileRef`, used by rendering and standard networking. `fetch(file.src).blob()` retains native file ranges through slicing and multipart construction. Native upload preparation copies those ranges into a native spool without copying the complete attachment into JavaScript. Explicit byte and text reads materialise content.

Completed recordings are retained independently until explicitly deleted. Apps persist file IDs and own reference counting and retention; disposing a screen does not delete accepted attachments.

Audio capture is imported from `@ink/audio/capture`; camera scanning is imported from `@ink/camera/scan`. Playback and still-photo imports exclude those optional native engines. MapLibre and other optional engines follow their module capability graph.

## Integration and verification

Custom Tabs launch, theme parameters and return handling are implemented. [Physical LP3 checks](verification-custom-tabs.md) passed website open/close, dark toolbar appearance and local OAuth sign-in/cancellation. Production providers still need checks with their registered redirects. Preserve browser security UI, and do not silently substitute the full browser or embedded OAuth when Custom Tabs is unavailable.

Validate the integrations against Weather, Passes, Index, Buses, Beeper, Spotify and Echo TV. Provider playback, background messaging and offline engines need their own integrations; component coverage alone does not replace them. Keep physical-device behaviour, emulator checks and provider-specific verification explicit in the corresponding evidence records.
