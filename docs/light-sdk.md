---
title: "LightOS"
description: "Tool entry, host preferences and available system services."
tag: "Design specification"
---

Enable host integration in the app configuration:

```toml
[lightos]
enabled = true
```

The build registers the app's entry points and required host capabilities. Ink coordinates tool launch, navigation, lifecycle and native services. Integration is versioned; the installed host determines which optional operations are available.

## Preferences and system actions

`@ink/lightos` exposes host version and capability information, account/server state where authorised, and supported system actions. Keyboard options and haptic preferences feed Ink's native interaction defaults.

```ts
import { lightos } from "@ink/lightos";

if (lightos.capabilities.openDialler) {
  await lightos.openDialler({ phoneNumber });
}
```

The example assumes a user-selected phone number. Use host operations for the dialler and supported ringtone selection. Restricted services can return unavailable, denied or blocked-by-host outcomes. A local Android permission does not override host policy.

Hardware keys flow through focus, navigation and active media ownership. Avoid separate app listeners competing with the keyboard or media session for the same key.

## Entry and recovery

A tool can open from the launcher, a link or a notification with a validated route. Cold launch reconstructs its account and records from storage. External inputs are decoded before navigation, and invalid or obsolete destinations lead to a controlled fallback.

Host/server data can change during the app's lifetime. Observe it as a snapshot and release subscriptions with their owner. Do not bake a signed-in user or a host permission into a build-time constant.

## Jobs, media and push

[Background jobs](background.md) register named handlers and durable inputs. [Audio](audio.md) keeps detached playback under a native media service and reconnects UI to existing state. Both must recover from Android termination.

Where push delivery is available, a provider adapter registers its transport, decodes the delivered bytes and persists enough information to request domain reconciliation. The app chooses its payload schema; Ink does not impose a universal notification envelope or backend registration protocol.

Push may be duplicated, delayed or unavailable. Use stable event IDs/cursors, account scoping and a durable sync strategy. A handler gets bounded headless execution, not an indefinitely running foreground app. UI presentation follows host capabilities and user preferences.

## Development and distribution

`ink info` reports host integration requirements. A development APK can expose unavailable states on an emulator or device lacking the host service. Inspect actual capabilities on the LP3 rather than inferring them from an emulator.

Installing an APK and qualifying for Light-approved distribution are separate processes. Distribution requirements and restricted service access must be agreed with Light; enabling this configuration does not confer approval.
