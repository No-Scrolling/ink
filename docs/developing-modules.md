---
title: "Develop an Ink module"
description: "Publish source packages and native Android capabilities for Ink apps."
tag: "Planned"
---

An Ink module is an npm package with an `ink` export. Start with a source module when existing Ink operations can implement the domain capability. Add a native adapter only for an Android SDK, platform API, background worker, hardware session, or directly manipulated native view.

## Choose a module type

| Requirement | Module type |
| --- | --- |
| Reusable components or screens | Source |
| HTTP provider client | Source |
| Pure domain transformation | Source |
| Device protocol over an existing low-level module | Source |
| Queryable local collections or bundled reference data | Source over Records |
| Android or vendor SDK | Native |
| Service, receiver, provider, or intent filter | Native |
| New hardware capability or native view | Native |

Do not add a native adapter only to run a JavaScript dependency. Ink apps contain no JavaScript runtime.

## Create a source module

```text
ink-weather/
├── package.json
├── preview/
│   └── scenarios.ts
└── src/
    ├── index.ts
    ├── domain.ts
    └── provider.ts
```

```json
{
  "name": "@example/ink-weather",
  "version": "1.0.0",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "ink": "./src/index.ts"
    }
  },
  "ink": {
    "preview": "./dist/preview.json"
  },
  "peerDependencies": {
    "ink": "*",
    "@ink/network": "*"
  }
}
```

The `types` condition supports editors. The `ink` condition gives the compiler source to validate and specialise. The app compiler does not execute the package's JavaScript entry point.

## Write compiler-compatible source

Source modules can:

- export typed functions, components, and zero-argument screens;
- declare resources, actions, sessions, and background plans from Ink modules;
- derive serialisable values with pure functions;
- map and combine resources;
- construct, filter, and zip lists;
- narrow tagged values and validate bounded values;
- call approved pure intrinsics such as date parsing and byte decoding.

They cannot:

- use Node.js, browser globals, `eval()`, dynamic imports, or runtime code generation;
- execute file, network, clock, or native I/O during compilation;
- capture arbitrary closures in resources or background work;
- depend on a package that requires a JavaScript runtime on the phone.

`ink package build` compiles named exports and pure functions into a versioned source-module IR. At app build time, Ink specialises the reachable call graph and links only the operations it reaches.

## Design a deep interface

Expose the domain operation an app needs:

```tsx
const forecast = OpenMeteo.forecast({ latitude, longitude, days: 7 });
```

Keep transport, decoding, caching, retries, permissions, credentials, and provider failures behind that interface. Do not require app callers to assemble an effect program, dependency layer, schedule, or scope.

Effect-style schemas, tagged errors, replaceable adapters, and acquisition scopes are useful implementation concepts. In Ink, source schemas and tagged errors provide validation, production and preview adapters occupy replaceable seams, and Rust ownership provides scopes.

## Choose a lifecycle form

Use:

| Form | Use |
| --- | --- |
| Resource | Load or observe one current value. |
| Action | Run explicit work and observe its completion. |
| Session | Own a stateful capability and ordered commands. |

A stream is a session with updates and no commands. A native view attaches to a session. Do not introduce another top-level lifecycle shape.

Resources use `loading`, `ready`, and `error`. Put usable stale values, refresh activity, and recoverable refresh failures on the ready value. Actions use `idle`, `running`, `success`, and `error`. Sessions publish complete domain snapshots.

## Return tagged errors

```ts
export type WeatherError =
  | { kind: "network"; message: string; retryable: true }
  | { kind: "rate-limited"; retryAtMs: number | null; message: string; retryable: true }
  | { kind: "invalid-response"; message: string; retryable: false };
```

Translate lower-level failures inside the module. Preserve provider payloads, HTTP status codes, and native exceptions only in redacted development diagnostics.

## Compose resource dependencies

Keep declarations stable. Pass a resource, session, or opaque reference as an operation dependency instead of declaring work conditionally:

```tsx
const account = oauthSession(options);

const profile = json<Profile>(url, {
  authorization: account.authorization,
});
```

The compiler records the dependency edge. Rust activates the consumer when the dependency becomes usable, reloads it when the reference generation changes, and disposes leases in dependency order.

Opaque references are nominal and non-serialisable. A module operation must declare each reference kind it can receive. Durable workers require durable references.

## Use Records for large local collections

Use [Records](records.md) when a module needs indexed queries, pagination, atomic multi-record updates, or a large bundled reference dataset. Declare collections, queries, and transactions inside the module, then export domain operations such as `Messaging.thread()` or `Transit.searchStops()`.

Do not expose raw collection access throughout app screens. Keeping query and migration knowledge inside the domain module improves locality and lets the module translate storage errors into its own tagged errors.

## Create a native module

Run:

```sh
ink module create @example/ink-battery
```

The generated package contains source bindings, a restricted module declaration, Android adapter code, and a preview adapter. The declaration is compiled during package development into a data-only manifest. App builds consume that manifest and never execute module build logic.

## Declare native operations

Use `defineNativeModule()` with literal configuration and Ink schemas:

```ts
export default defineNativeModule({
  name: "@example/ink-battery",
  namespace: "battery",
  resources: {
    status: {
      input: Schema.Void,
      output: BatteryState,
      errors: BatteryError,
      owner: "screen",
      requires: ["battery.read"],
    },
  },
  sessions: {
    changes: {
      input: Schema.Void,
      state: BatteryState,
      errors: BatteryError,
      owner: "screen",
      updates: { delivery: "latest" },
      commands: {},
      requires: ["battery.observe"],
    },
  },
});
```

The manifest supports resources, actions, sessions, native views, workers, opaque handle kinds, and operation-level requirements. It does not accept dynamic operation generation or arbitrary Android configuration.

## Define ownership and delivery

Each resource or session declares `owner: "screen" | "application"`. Use application ownership only when the public interface promises work that survives navigation.

Update delivery is:

- `latest` for replaceable state such as a sensor snapshot;
- a bounded queue with `drop-oldest` and dropped counts for lossy events;
- a bounded queue with `error` when losing a protocol event is unsafe.

Document cancellation truthfully. Rust always stops observation and ignores late output. Immediate termination of Android, codec, or network work is best effort unless the adapter contract explicitly guarantees it.

## Define handles and grants

A module can declare nominal handles such as `FileReference`, `StoredSecret`, or a package-owned device session. Handles include namespace, generation, lifetime, and serialisability metadata.

An operation lists the handle kinds it accepts and the access it needs. Android receives a scoped grant, not a path, URI, credential string, or global handle registry. Work that can survive process death accepts only durable references.

## Add a native view

A view declares props, events, its attached session, sizing, focus, and accessibility contract:

```ts
views: {
  map: {
    session: "map",
    props: MapProps,
    events: MapEvents,
    sizing: "fill",
    accessibility: "native-tree",
    eventDelivery: "coalesced",
    requires: ["maps.render"],
  },
}
```

Use `native-tree` when the adapter owns complete semantics. Use declared semantic nodes and actions when Ink must expose them. Every view specifies reduced-motion, text-scale, focus restoration, and event-coalescing behaviour.

## Add a background worker

A native worker receives a compiler-validated serialisable plan and durable grants. It cannot receive screen state, screen-owned sessions, or temporary handles.

The worker returns a typed result to Background, which owns scheduling, retries, idempotency policy, and durable result state. The adapter owns only execution of its declared operation.

## Attach Android integration to operations

Declare validated artefact groups containing Maven dependencies, AAR files, permissions, features, components, resources, package visibility, redirect intent filters, provider authorities, and foreground-service types.

Attach each group to the operations that require it. Importing an unrelated operation must not include the group. Modules cannot inject arbitrary manifest XML or Gradle scripts.

## Generate adapter contracts

Run:

```sh
ink module build
```

The command validates source and schemas, emits TypeScript declarations and source IR, generates Kotlin types and adapter interfaces, writes the versioned data manifest, builds native artefacts, and validates preview adapters.

Rust owns lifecycle, ordering, cancellation, handle leases, and schema validation. Android adapters own platform APIs and vendor SDKs. Generated registries connect manifest operation IDs to adapters; a central hand-written module switch is not part of the extension interface.

## Add production and preview adapters

Every true external dependency has at least two adapters: production and deterministic preview. A source provider supplies response fixtures and error scenarios. A native module implements the generated interface for Android and preview.

Include unavailable, denied, empty, malformed, interrupted, and overflow scenarios when the interface can produce them. Preview adapters are development artefacts and are not included in the APK.

## Inspect and publish

Run:

```sh
ink module build
ink module verify
ink module inspect .
npm publish
```

`ink module inspect` shows public operations, ownership, grants, permissions, components, network hosts, artefact groups, native size, and preview scenarios. `ink module verify` checks generated hashes and rejects undeclared build hooks.

Follow semantic versioning for the app-facing interface and manifest contract. Keep modules focused on complete domain operations rather than exposing a wide wrapper around an SDK.

Read [Build an Open-Meteo module](open-meteo-module.md) for a source-module example.
