---
title: "Third-party modules"
description: "Install Ink modules that add screens, data sources, and native device capabilities."
tag: "Planned"
---

Ink modules are npm packages compiled into your app. They can provide components, screens, data sources, or native device capabilities without adding a JavaScript runtime.

## Install a module

Install a module with your package manager:

```sh
npm install @example/ink-open-meteo
```

Import it from TypeScript or TSX:

```tsx
import { OpenMeteo, OpenMeteoAttribution } from "@example/ink-open-meteo";
import { Screen, Stack, Text, match } from "ink";

export default function Weather() {
  const forecast = OpenMeteo.forecast({
    latitude: 51.5072,
    longitude: -0.1276,
    units: "metric",
  });

  return (
    <Screen title="Weather">
      {match(forecast, {
        loading: () => <Text>Loading forecast</Text>,
        ready: ({ value }) => (
          <Stack gap={12}>
            <Text>{value.current.temperature}°</Text>
            <OpenMeteoAttribution />
          </Stack>
        ),
        error: ({ error }) => <Text>{error.message}</Text>,
      })}
    </Screen>
  );
}
```

Ink resolves the module's `ink` export and compiles it with the rest of the app. The package's JavaScript entry point is not executed on the phone.

## Module types

Ink supports two module types.

| Type | Use |
| --- | --- |
| Source module | Composes Ink components and existing modules with TypeScript or TSX. |
| Native module | Adds an Android SDK, platform API, background component, or native view. |

A weather API client, reusable settings screen, or domain-specific network package is a source module. A maps SDK, health sensor integration, or vendor payment terminal is a native module.

Both types expose an ordinary typed import. App code does not use a bridge API or distinguish native results from other Ink resources and controllers.

## Check module requirements

Run `ink info` after installing a module:

```sh
ink info
```

The output lists:

- linked modules and versions;
- native operations used by the app;
- Android permissions and their reasons;
- services, receivers, providers, and intent filters;
- bundled SDKs and their estimated size;
- app-scoped work such as playback or background tasks.

Ink includes only capabilities reached from your app's imports. Importing one component does not automatically include every capability in its package.

## Permissions

A native module declares each Android permission with a reason and the operation that uses it. Ink merges those declarations into the app and reports conflicts during `ink check`.

The module controls when a permission is required, but your screen controls when to start an action that prompts the user.

```tsx
const scan = Barcode.scanner({ formats: ["qr"] });

<Button onPress={() => scan.start()}>Scan code</Button>
```

Installing a package does not grant a permission. Android still applies its normal permission and settings behaviour.

## Resource lifecycle

Module resources, streams, controllers, and native views follow Ink's normal screen lifecycle:

- resources load when their screen becomes active;
- streams subscribe while their owner is active;
- controllers release their native session when their owner is disposed;
- leaving a screen cancels work that belongs to it;
- late results from cancelled work are ignored.

A module can expose an app-scoped capability when work must survive navigation. The method name and documentation identify that behaviour; app code does not pass a generic scope object.

## Handle module errors

Modules return tagged errors that you can narrow with `match()`.

```tsx
{match(document, {
  loading: () => <Text>Opening document</Text>,
  ready: ({ value }) => <Reader document={value} />,
  error: ({ error }) => (
    <Text>
      {error.kind === "unsupported-format"
        ? "This document format is not supported"
        : error.message}
    </Text>
  ),
})}
```

Module errors do not expose Android exception classes, HTTP client objects, or stack traces. Use `ink logs` to inspect development diagnostics for the failing operation.

## Preview native modules

Native modules include a preview adapter with deterministic data. The preview toolbar lists the scenarios supplied by the package, such as `connected`, `permission-denied`, or `empty`.

Preview adapters run on the development host and are not included in the APK. If a module does not provide a preview adapter, its native view displays an unavailable state and `ink check` reports the missing development capability.

## Module compatibility

An Ink module declares the compiler and native extension API versions it supports. Installation fails before Android compilation when the package requires a newer incompatible version.

Keep modules in your lockfile. Ink records the native manifest and artefact hashes so the same dependency graph produces the same linked capabilities.

## Review a native module

Native code has the same authority as other code in the APK. Before adding a third-party native module, review:

- its permissions and exported Android components;
- its bundled SDKs and native libraries;
- its network and data-handling documentation;
- its licence and update policy;
- the information shown by `ink module inspect <package>`.

Ink validates declared integration, but it does not sandbox a native adapter from the rest of the Android process.

Read [Develop an Ink module](developing-modules.md) to publish a source or native module.
