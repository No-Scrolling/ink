---
title: "System"
description: "Native links, sharing, appearance and application lifecycle."
tag: "Planned"
---

> **Planned.** This package is not implemented. The APIs below describe the proposed design.

`@ink/system` exposes a small set of app-level interactions with Android and the host.

```ts
import { system } from "@ink/system";

await system.openURL("https://example.com/help");
await system.share({ text: "Meet at the station at 18:00" });
```

Calls return promises and distinguish cancellation, unavailable handlers and failure. Validate externally supplied URLs and restrict schemes appropriate to your feature. Opening an external activity keeps its own operation alive through the temporary app pause.

Use managed file references for file sharing. Native code grants temporary access to the receiving app; do not manufacture a raw file URI. Sharing a document can expose its contents to the chosen recipient.

## Preferences and lifecycle

`system.lifecycle` is a planned readable snapshot. App appearance is already provided by `setColourScheme` and `useColourScheme` from `ink`; see [App appearance](ink.md#app-appearance). Observe lifecycle through the framework's visibility hooks for screen work; use the app snapshot only when a domain service needs it.

`system.haptic(kind)` respects availability and host preferences. Avoid haptics from repeated render or progress callbacks. Keyboard and Back behaviour are integrated into Ink's focus and navigation model.

Permissions belong to the package requesting a capability. Ask at the relevant user action, expose denial and explain a route to settings when the OS requires it. A manifest declaration alone does not grant runtime permission.

[LightOS](light-sdk.md) covers host-specific services such as the dialler, ringtone selection and tool entry. Feature-detect optional host services and provide a controlled unavailable state.
