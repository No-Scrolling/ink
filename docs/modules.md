---
title: "Third-party modules"
description: "Install compiled Ink modules that add domain operations and native capabilities."
tag: "Planned"
---

Ink modules are npm packages compiled into your app. They can provide components, domain resources, actions, sessions, background plans, or native views without adding a JavaScript runtime.

## Install a module

Install a module with your package manager:

```sh
npm install @example/ink-open-meteo
```

Import its domain interface:

```tsx
import { OpenMeteo } from "@example/ink-open-meteo";
import { Screen, Text, match } from "ink";

const forecast = OpenMeteo.forecast({
  latitude: 51.5072,
  longitude: -0.1276,
  days: 7,
});

<Screen title="Weather">
  {match(forecast, {
    loading: () => <Text>Loading forecast</Text>,
    ready: ({ value }) => <Text>{value.current.temperature}°</Text>,
    error: ({ error }) => <Text>{error.message}</Text>,
  })}
</Screen>
```

Ink resolves the package's `ink` export and specialises reachable source operations into `app.ink`. Package JavaScript is not executed on the phone.

## Choose a module type

| Type | Use |
| --- | --- |
| Source module | Composes components, pure transformations, and existing Ink operations. |
| Native module | Adds an Android SDK, platform API, worker, hardware session, or native view. |

A weather provider, device-specific Bluetooth protocol, or reusable settings screen can be a source module. A maps renderer, vendor payment SDK, or new hardware integration requires a native module.

Both module types expose ordinary typed imports. App code does not use bridge calls, manifests, dependency layers, or native operation names.

## Use consistent lifecycle shapes

Modules expose three lifecycle forms:

- a **resource** loads one current value with `loading`, `ready`, and `error`;
- an **action** runs explicit work with `idle`, `running`, `success`, and `error`;
- a **session** publishes one domain state snapshot and ordered commands.

A ready resource can include freshness, background activity, and a warning without becoming another status. A stream is a session without commands. A native view attaches to a session.

Resources and sessions declare screen or application ownership in the module. Ordinary callers do not pass a generic scope object.

## Compose modules with opaque references

Modules can pass nominal references such as authorisations, secrets, files, bytes, or map content without exposing their contents.

References cannot enter ordinary state, route data, persisted values, or serialised results. Rust owns their namespace, generation, leases, and disposal. Work that survives process death must use a durable reference.

Installing a package does not grant it general access to every reference. Each operation declares the reference kinds it can receive.

## Inspect linked requirements

Run:

```sh
ink info
```

The output explains the complete reason chain from an app import to:

- linked source and native operations;
- Android permissions, components, and foreground-service types;
- bundled SDK artefacts and estimated size;
- application-scoped sessions, workers, and durable data;
- network hosts and opaque capability grants.

Reachability is operation-level. Importing barcode generation does not include Camera, and importing a static map does not include Location. Shared native artefacts can form a capability group when an SDK cannot be split further.

## Handle permissions and availability

The module that owns a capability also owns its permission and availability interface. A Barcode scanner can share Camera implementation internally without making the app import Camera permission.

Creating a module value never opens a runtime permission prompt. A standard native view can render an explicit permission action, or a custom app interface can call the module's permission command from a user action.

Packaging is not runtime availability. Use `ink info` to inspect what was linked and the owning module to inspect device or policy availability.

## Handle module errors

Modules return small tagged errors in domain language. They do not expose Android exceptions, HTTP client values, provider payloads, or stack traces.

`retryable` appears only when the owning resource, action, or session provides a truthful retry operation. Retry times use absolute `retryAtMs` values.

## Preview modules

Every operation that depends on a platform or true external provider supplies deterministic preview scenarios. Source provider modules include fixtures for success, empty, malformed, offline, and domain error cases. Native modules implement the same generated contract with a preview adapter.

Preview adapters run on the development host and are not included in the APK. A missing adapter produces an unavailable preview state and a warning from `ink check`.

## Check compatibility and trust

An Ink module declares compatible compiler, module IR, and native extension versions. The app compiler consumes a versioned data-only manifest; it never runs package build hooks.

Keep modules in your lockfile. Ink records source, manifest, and native artefact hashes so the same dependency graph links the same capabilities.

Native code has the same authority as other code in the APK. Before installing it, review its permissions, Android components, SDKs, network behaviour, licence, update policy, and `ink module inspect` output.

Read [Develop an Ink module](developing-modules.md) to create a source or native package.
