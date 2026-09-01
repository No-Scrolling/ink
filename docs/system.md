---
title: "System"
description: "Open settings, links, the dialler, and email through Android."
tag: "Planned"
---

`@ink/system` opens supported Android settings and external app surfaces. Capability availability belongs to the module that owns the capability rather than a central string registry.

## Open settings

Create a system action and call `run()` from a user action:

```tsx
import { systemAction } from "@ink/system";
import { Button, Text, match } from "ink";

const settings = systemAction({
  kind: "settings",
  page: "notifications",
});

<Button onPress={() => settings.run()}>Notification settings</Button>

{match(settings, {
  idle: () => null,
  running: () => <Text>Opening settings</Text>,
  success: () => null,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

Settings pages are `"app"`, `"notifications"`, `"location"`, `"bluetooth"`, and `"network"`. Opening settings does not report whether the user changed a setting. Modules refresh relevant permission or availability state when the app returns.

## Open a link, dialler, or email

Pass one target to `systemAction()`:

```tsx
const website = systemAction({
  kind: "web",
  url: "https://example.com/help",
});

const phone = systemAction({
  kind: "dial",
  number: "+44 20 7946 0958",
});

const email = systemAction({
  kind: "email",
  to: "support@example.com",
  subject: "Support request",
});
```

Web URLs must use HTTPS. Opening the dialler fills the number but does not place a call. Email fields open in the user's chosen email app and do not send a message automatically.

When several apps can handle a target, Android shows its chooser. If none can handle it, the action returns `no-handler`.

## Check module availability

Use the interface belonging to the capability. For example, Camera reports camera availability and Connectivity reports connection state. This allows first- and third-party modules to define truthful domain reasons without registering names in System.

Use `ink info` to inspect whether an operation is linked into an app and why. Packaging is a compiler fact, not a runtime System capability.

## Lifecycle and errors

A system action uses `idle`, `running`, `success`, and `error`. Calling `run()` while it is running has no effect. Leaving the screen stops observing the result but does not close a surface that already opened.

Errors provide `kind`, `message`, and `retryable`. They distinguish invalid targets, missing handlers, unsupported settings, device-policy restrictions, cancellation, and unexpected failures. The package requests no runtime permissions.
