---
title: "Publish a package"
description: "Write ordinary TypeScript packages and add native code when a capability needs it."
tag: "Design specification"
---

Start with the operation an app needs: `getForecast`, `sendMessage`, `searchStops` or `connectScale`. Hide request formats, SDK callbacks and persistence details behind that operation. A package should remove work from its callers rather than require them to assemble a small framework.

## A TypeScript package

```json
{
  "name": "@example/ink-weather",
  "version": "1.0.0",
  "type": "module",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "import": "./dist/index.js"
    }
  },
  "files": ["dist"],
  "sideEffects": false
}
```

Use your normal TypeScript bundler to produce these files. Set `sideEffects: false` only when importing the package has no required side effects. A UI package declares `react` and `ink` as peer dependencies and compiles TSX against `react/jsx-runtime`. Use the supported peer-version ranges; do not bundle a second copy of React or Ink.

```ts
export type Forecast = { temperature: number; observedAt: string };
export type WeatherClient = {
  getForecast(place: { latitude: number; longitude: number },
    options?: { signal?: AbortSignal }): Promise<Forecast>;
};
```

Accept a transport or domain dependency when it actually varies. A production HTTP client and an offline preview fixture can satisfy this same interface. Keep UI hooks in an optional `/ui` entry point so headless work does not import screen code.

## Data and failures

Decode provider responses from `unknown` with ordinary code or a chosen schema package. Return useful domain values rather than exposing raw response shapes everywhere. Throw an `Error` subclass with stable codes for failures that callers can handle; preserve the cause for diagnostics without copying tokens or complete payloads into messages.

Document cache scope, staleness, cancellation and mutation idempotency. A generic `Promise<T>` does not tell a caller whether cancellation stops observation, cancels a transfer or rolls back a write.

Use normal `Uint8Array` and `DataView` for binary protocols. Use [Records](records.md) for indexed data, [Files](files.md) for large content and [Secure store](secure-store.md) for persisted credentials.

## A native package

```sh
ink module create @example/ink-battery
```

The scaffold contains TypeScript bindings, an Android library, an `ink-module.json` manifest and a development fixture. Native code may use Kotlin for Android APIs or Rust/C++ for an engine or codec. Its manifest registers the platform integration; its normal package exports remain the app-facing interface.

```json
{
  "schemaVersion": 1,
  "name": "@example/ink-battery",
  "abi": 1,
  "android": {
    "library": "./android",
    "registration": "com.example.inkbattery.BatteryModule",
    "permissions": []
  },
  "bindings": "./native.contract.json"
}
```

Paths are relative to the package. A bindings contract declares operation names, serialisable inputs/outputs, errors, handles and events. Code generation emits matching TypeScript and Kotlin/Rust transport types. These transport schemas are an internal native seam, not an application-wide schema dependency.

`ink module build` generates bindings and builds the native artefact. `ink module inspect` reports registration, ABI, dependencies and permissions. `ink check` rejects an incompatible ABI or missing declared native entry point.

## Async native calls

Native methods return completions to the JavaScript event loop. Perform disk, network and codec work away from the UI thread. A cancellation token accompanies cancellable operations; cancel promptly and tag replies so a late result cannot target a replaced call.

Returned data is validated and copied using the documented transport types. Primitive values, JSON-shaped records and typed-array payloads are supported. Large files and images cross as native references. Do not expose arbitrary pointers, JNI objects or filesystem paths as an application contract.

Each native controller has explicit activation and disposal. A UI hook constructs an inert descriptor during render, activates it after commit, and releases its attachment when hidden. React can abandon a render, and development Strict Mode can repeat setup and cleanup. Keep acquisition out of render, make disposal idempotent, and make reactivation safe. Reopening reconciles the current native state. A controller acquired imperatively has an explicit `close()` operation that callers use in `finally`.

## Events and views

Declare whether an event is a replaceable snapshot or an ordered delta. Coalesce snapshots before crossing into JavaScript. Bound ordered queues, expose sequence gaps and fail or resynchronise on overflow. Do not silently convert a message stream into latest-only delivery.

A native view owns rendering and gestures for its specialised surface. It declares sizing, props, events, accessibility semantics and lifecycle. Camera previews, maps and video are examples. It must cooperate with Ink's clipping, navigation, focus and input ownership; it cannot assume a browser DOM.

A detached player or transfer service has a lifetime independent of its UI controller. Releasing a controller disconnects observation. Stopping the underlying operation is an explicit command. Document Android termination and recovery separately.

## Build integration

Declare native dependencies and Android permissions/components in the package's integration manifest. Use capability groups or subpath entry points where the underlying library is separable. Native packages may need conventional Gradle/Cargo builds; those builds execute code on the development host and are part of the dependency trust model.

## Publish

```sh
ink module build
ink module inspect .
npm pack
npm publish
```

For a pure TypeScript package, use its normal build command instead of `ink module build`. Inspect the packed files, declare licences for bundled native artefacts and keep versions reproducible. Examples should exercise installation of the packed package as well as local workspace imports. Device verification remains necessary for permission, lifecycle and service behaviour.
