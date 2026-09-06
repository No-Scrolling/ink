---
title: "Product design"
description: "The intended Ink experience and the work needed to reach it."
---

Ink is a React and TypeScript framework for Light Phone III apps. Developers supply app data and behaviour; Ink supplies the phone's visual language, native interaction and device integration.

This is the product design reference. Module guides describe current interfaces and explicitly mark intended interfaces that are not implemented yet. Historical plans and verification records describe work at the time, not additional product requirements.

## Authoring

Functions, hooks, context, composition, conditional rendering and array mapping use ordinary React semantics. Native requirements come from imported modules, not special rules about application expressions. Use standard networking and compatible JavaScript libraries.

Ink does not promise React Native native-module, DOM or Node compatibility. Its runtime facilities are listed in [runtime compatibility](runtime-compatibility.md).

## Screens and behaviour

Screen owns navigation chrome, spacing, scrolling and keyboard accommodation. Row, PlayingScreen and ConversationScreen hide shared layout and interaction behind data and callbacks. Apps own provider requests, playback engines, message delivery and persistence.

Keep ordinary composition available through Text, Image, Button, Stack and List. Every List is virtualised and measures its content automatically. Ink triggers pagination; the app fetches and appends or prepends data. ConversationScreen additionally owns message actions, reply selection and keyboard/scroll coordination.

Do not turn each app layout into a framework feature. Weather columns, album detail composition and saved-location screens can use existing primitives.

## Native modules

Commands return promises. Continuing work has an explicit owner; React hooks manage screen-owned work. Observations use the existing loading/ready/error snapshot shape. Persistent operations such as downloads survive observers disappearing.

Keep the simple operation simple. Clipboard and Secure Store expose a few commands. Location separates a one-off fix from ongoing tracking. Background jobs use durable input rather than captured foreground state.

Include capabilities through explicit module imports. Optional engines and capture features must not increase every app's native footprint. Reuse native engines behind Ink interfaces; provider-specific behaviour stays in provider integrations.

## Remaining product scope

| Capability | Intended scope |
| --- | --- |
| [Auth](auth.md) | Browser and device-code sign-in, session observation, coordinated refresh and local sign-out. |
| [Secure Store](secure-store.md) | Small encrypted string values, shared by Auth and app code. |
| [Files and media](files.md) | One photo/video picker, document selection, shared durable file references, image preparation, saving and sharing. |
| [Maps](maps.md) | Optional MapLibre view, markers, native gestures and explicit camera movement. |
| [Store](store.md) | Keep JSON storage; add direct read-only SQLite access for bundled databases first. |
| [Connectivity](connectivity.md) | Observe network state; request success remains an HTTP concern. |
| [Downloads](downloads.md) | Durable ordinary HTTP files; provider download engines remain separate. |
| External actions | Web links and browser sign-in use a browser-backed in-app window, keeping a direct return to LightOS. Text sharing uses a small framework command; file sharing belongs with files and media. |

Reader, Bluetooth and Sensors are outside scope. There is no separate Records or System module. Map clustering, offline map regions, general image editing and video playback are not part of this scope.

## Implementation gaps

The existing screen components remain the intended design. The following changes still require implementation:

- Implement the capabilities above that are marked planned.
- Use one managed FileRef across capture, picking, recording, download, rendering and upload. Native-backed file bodies must avoid whole-file JavaScript copies.
- Retain completed recordings independently. The current recorder replaces its previous recording.
- Split audio capture and camera scanning into optional entry points. Current imports still include broader native groups.
- Implement and verify Custom Tabs launch, appearance and return behaviour. The inspected LP3 Chromium [advertises support](verification-custom-tabs.md). Preserve browser security UI; do not silently fall back to the full browser or embedded OAuth.

These are design decisions, not claims that the corresponding code has shipped. Validate the integrations against Weather, Passes, Index, Buses, Beeper, Spotify and Echo TV. Provider playback, background messaging and offline engines need their own integrations; component coverage alone does not replace them.
