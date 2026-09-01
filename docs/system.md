---
title: "System"
description: "Check device capabilities and open settings, links, the dialler, and email."
---

`@ink/system` checks whether Ink capabilities are available and opens supported settings and apps.

## Check a capability

`systemCapability()` returns whether a packaged capability is available on the current device.

```tsx
import { systemCapability } from "@ink/system";
import { Button, Text, match } from "ink";

const scanning = systemCapability("barcode.scan");

{match(scanning, {
  loading: () => <Text>Checking scanner</Text>,
  ready: (result) => result.value.status === "available" ? (
    <Button href="/scan">Scan a code</Button>
  ) : (
    <Text>Scanning is not available</Text>
  ),
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

An unavailable result includes one reason:

| Reason | Meaning |
| --- | --- |
| `"missing-hardware"` | The device lacks required hardware. |
| `"missing-software"` | The device software does not support the capability. |
| `"disabled-by-policy"` | Device policy prevents access. |
| `"not-packaged"` | The app does not include the capability. |

Supported capability names are:

- `"files.import"`, `"files.export"`, and `"files.share"`;
- `"auth.browser"`;
- `"background.work"`;
- `"notifications.local"` and `"notifications.remote"`;
- `"location.fix"`;
- `"maps.render"`;
- `"camera.photo"` and `"camera.video"`;
- `"barcode.scan"` and `"barcode.generate"`;
- `"audio.playback"` and `"audio.recording"`;
- `"media.picker"` and `"media.video"`;
- `"sensors.accelerometer"`, `"sensors.gyroscope"`, and the other sensor names accepted by `@ink/sensors`;
- `"bluetooth.le"`;
- `"reader.epub"`, `"reader.text"`, and `"reader.pdf"`;
- `"charts.render"`.

Availability does not include permission state. For example, `"camera.photo"` can be available while camera permission is denied. Read permission through the package that owns the capability.

## Open settings

Create a system action for the settings page, then call `run()` from a user action.

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
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

Settings pages are `"app"`, `"notifications"`, and `"location"`. Opening settings does not report whether the user changed a setting. Permission resources reload when the app returns to the foreground.

## Open a link, dialler, or email

Pass one target to `systemAction()`:

```tsx
import { systemAction } from "@ink/system";
import { Button } from "ink";

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

<Button onPress={() => website.run()}>Open help</Button>
<Button onPress={() => phone.run()}>Open dialler</Button>
<Button onPress={() => email.run()}>Email support</Button>
```

Web URLs must use HTTPS. Opening the dialler fills the number but does not place a call. Email fields open in the user's chosen email app and do not send a message automatically.

When several apps can handle a target, the device shows its chooser. If none can handle it, the action returns a `no-handler` error.

## Lifecycle and errors

Capability resources reload when the app returns to the foreground. They do not poll while the app is in the background.

A system action is `idle`, `running`, `success`, or `error`. Calling `run()` while the same action is running has no effect. Leaving the screen stops observing the result but does not close a surface that has already opened.

Errors provide `kind`, `message`, and `retryable`. They distinguish unsupported capabilities, invalid targets, missing handlers, device-policy restrictions, cancellation, and unexpected failures. The package requests no runtime permissions.
