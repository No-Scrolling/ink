---
title: "Develop an Ink module"
description: "Publish reusable TypeScript packages and native Android capabilities for Ink apps."
---

An Ink module is an npm package with an `ink` export. Start with a source module when you can build the capability from existing Ink APIs. Add a native adapter only when you need an Android SDK, platform component, hardware API, or native view.

## Choose a module type

Use this decision table before creating the package.

| Requirement | Module type |
| --- | --- |
| Reusable components or screens | Source |
| HTTP API client | Source |
| Domain logic over existing Ink resources | Source |
| Android or vendor SDK | Native |
| Service, receiver, provider, or intent filter | Native |
| New hardware capability | Native |
| New directly manipulated view | Native |

Source modules remain smaller and work in preview without a platform implementation. Do not add a native adapter to run a JavaScript-only dependency; Ink apps do not contain a JavaScript runtime.

## Create a source module

Create a package with source, declarations, and an `ink` export:

```text
ink-weather/
├── package.json
└── src/
    ├── index.ts
    ├── domain.ts
    └── WeatherCard.tsx
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
  "peerDependencies": {
    "ink": "*",
    "@ink/network": "*"
  }
}
```

The `types` condition supports TypeScript editors. The `ink` condition gives the compiler the source it validates and links into `app.ink`.

## Write compiler-compatible source

Module source uses the same TypeScript and TSX subset as an app. It can:

- export components and zero-argument screens;
- call resources, actions, streams, and controllers from Ink modules;
- derive serialisable values with pure functions;
- map a resource value or tagged error;
- combine resources;
- set static defaults and hide provider-specific options.

It cannot:

- import Node.js built-ins;
- use `eval()`, dynamic imports, or runtime code generation;
- perform file or network I/O while the compiler resolves the package;
- depend on a JavaScript package that expects a browser or Node.js runtime;
- read arbitrary environment variables from package initialisation.

The compiler reports unsupported source at the package file and line that uses it.

Build the package declarations before publishing:

```sh
ink package build
```

The command validates every exported Ink source file, generates `dist` declarations, and checks that the `types` and `ink` export targets exist. It does not bundle or transpile source for a JavaScript runtime.

## Design the public interface

Expose the domain operation an app needs. Keep transport, cache, permission, and platform orchestration inside the module.

```ts
const forecast = OpenMeteo.forecast({ latitude, longitude });
```

Do not make every screen assemble the same workflow:

```ts
// Avoid this as an app-facing interface.
const forecast = Resource.make(
  "Forecast.load",
  Effect.gen(function* () {
    // Provider lookup, decoding, retry, and cache policy.
  }),
);
```

Effect-style ideas are still useful inside Ink: typed dependencies, tagged errors, schemas, scopes, and retry policy all make modules more reliable. The module interface should absorb that complexity and return one resource, action, stream, controller, or view.

## Use capability shapes

Choose the shape that matches the capability's lifecycle.

| Shape | Use |
| --- | --- |
| Resource | Load one current value. |
| Action | Run explicit work when the caller invokes it. |
| Stream | Receive repeated values or events. |
| Controller | Own state and commands for one native session. |
| Native view | Draw and handle direct manipulation through Android. |

Do not model a stateful media player as unrelated actions, or model a one-time file export as a long-lived controller.

## Return tagged errors

Give callers a small error union in domain language:

```ts
export type WeatherError =
  | { kind: "network"; message: string; retryable: true }
  | { kind: "rate-limited"; retryAfterMs: number | null; message: string; retryable: true }
  | { kind: "invalid-response"; message: string; retryable: true };
```

Translate lower-level failures inside the module. Preserve provider status codes and native exceptions in development diagnostics, not in the public error type.

## Develop against the package

Add an example app to the repository and install the package through a workspace or local file dependency. Run:

```sh
ink check
ink preview
ink info
```

`ink check` validates package source and schemas. `ink preview` loads source-module behaviour and native preview adapters. `ink info` shows which operations, permissions, and native artefacts the example app links.

## Create a native module

Run the module generator when existing packages cannot provide the capability:

```sh
ink module create @example/ink-battery
```

The command creates:

```text
ink-battery/
├── package.json
├── src/
│   ├── index.ts
│   └── module.ts
├── android/
│   ├── build.gradle.kts
│   └── src/main/kotlin/example/ink/battery/BatteryAdapter.kt
└── preview/
    └── adapter.ts
```

`src/module.ts` declares the complete compiler-to-native contract. Ink reads the declaration statically; it does not execute it as JavaScript.

## Declare a native interface

Use `defineNativeModule()` and Ink schemas:

```ts
import { Schema, defineNativeModule } from "@ink/native";

const BatteryState = Schema.Struct({
  level: Schema.Number.pipe(Schema.between(0, 1)),
  charging: Schema.Boolean,
  lowPowerMode: Schema.Boolean,
});

const BatteryError = Schema.TaggedUnion({
  unavailable: { message: Schema.String },
  failed: { message: Schema.String, retryable: Schema.Boolean },
});

export default defineNativeModule({
  name: "@example/ink-battery",
  namespace: "battery",
  resources: {
    status: {
      input: Schema.Void,
      output: BatteryState,
      errors: BatteryError,
    },
  },
  streams: {
    changes: {
      input: Schema.Void,
      output: BatteryState,
      errors: BatteryError,
      overflow: "latest",
    },
  },
});
```

The declaration accepts exported schemas and literal configuration. It cannot inspect the machine, access the network, or generate operations dynamically.

## Export the app interface

Wrap generated bindings in product language:

```ts
import battery from "./module";

export const Battery = {
  status: () => battery.status(),
  changes: () => battery.changes(),
};
```

Apps use the result like any other Ink resource:

```tsx
const battery = Battery.status();

{match(battery, {
  loading: () => <Text>Reading battery</Text>,
  ready: ({ value }) => <Text>{Math.round(value.level * 100)}%</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

Do not export the generated binding directly when a smaller domain interface can hide operation names, defaults, or SDK details.

## Generate the adapter contract

Run:

```sh
ink module build
```

The command:

1. validates the module declaration;
2. generates TypeScript declarations;
3. generates Kotlin input, output, error, and adapter types;
4. writes the versioned native module manifest;
5. builds the Android artefact;
6. validates the preview adapter against the same contract.

Generated Kotlin exposes a narrow interface:

```kotlin
interface BatteryModuleAdapter {
    suspend fun status(
        context: InkResourceContext,
        input: Unit,
    ): InkResult<BatteryState, BatteryError>

    fun changes(
        context: InkStreamContext,
        input: Unit,
        emit: (InkResult<BatteryState, BatteryError>) -> Unit,
    ): InkSubscription
}
```

The context provides cancellation, approved platform services, and redacted diagnostics. It does not expose the Ink engine or UI tree.

## Understand the native boundary

Ink links a native module in four layers:

```text
TypeScript import and generated types
                ↓
Compiler-validated module manifest
                ↓
app.ink operation IDs and Rust lifecycle
                ↓
Generated Android registry and module adapter
```

The compiler collects only imported operations, assigns compact module and operation IDs, and adds their declared permissions and native artefacts to the Android project.

The Rust runtime owns resource states, controller ordering, cancellation, and disposal. It sends schema-encoded inputs to the generated Android registry. The registry selects the adapter and returns a schema-checked success or tagged error.

No layer evaluates package JavaScript on the phone. Source modules stop at the first two layers because their work lowers to capabilities that the app already includes.

## Implement the Android adapter

Implement the generated interface and mark the entry point:

```kotlin
@InkModuleAdapter("@example/ink-battery")
class BatteryAdapter(
    private val manager: BatteryManager,
) : BatteryModuleAdapter {
    override suspend fun status(
        context: InkResourceContext,
        input: Unit,
    ): InkResult<BatteryState, BatteryError> =
        InkResult.Success(readBatteryState(manager))

    override fun changes(
        context: InkStreamContext,
        input: Unit,
        emit: (InkResult<BatteryState, BatteryError>) -> Unit,
    ): InkSubscription = observeBattery(context, emit)
}
```

The adapter must:

- stop work when the context is cancelled;
- complete each resource or action once;
- emit only values accepted by the generated schema;
- translate Android and SDK failures into declared errors;
- release listeners, views, files, and sessions when disposed;
- avoid logging fields marked as sensitive.

Ink selects the operation dispatcher and serialises controller commands. Do not create an unbounded worker pool or a second lifecycle inside the adapter.

## Add a controller

Use a controller when native operations share a session:

```ts
controllers: {
  player: {
    input: PlayerOptions,
    state: PlayerState,
    errors: PlayerError,
    commands: {
      play: { input: Schema.Void },
      pause: { input: Schema.Void },
      seek: { input: Schema.Struct({ positionMs: Schema.Int }) },
    },
  },
}
```

Ink creates one adapter instance per controller, orders its commands, and disposes it with its owner. Publish complete state snapshots so the runtime does not need to reproduce SDK state transitions.

## Add a native view

Declare serialisable props and events:

```ts
views: {
  map: {
    props: MapProps,
    events: {
      cameraChanged: CameraState,
      annotationPressed: AnnotationId,
    },
    controller: "mapController",
  },
}
```

Ink owns layout and mounting. The adapter owns drawing, SDK view lifecycle, and direct gestures. Send application events through declared callbacks rather than mutating the rest of the UI tree.

## Declare Android integration

Add validated Android requirements to `module.ts`:

```ts
android: {
  adapter: "example.ink.battery.BatteryAdapter",
  minSdk: 28,
  permissions: [],
  maven: [],
  components: [],
  resources: ["res/xml/battery_defaults.xml"],
}
```

You can declare permissions, hardware features, Maven dependencies, bundled AAR files, services, receivers, providers, intent filters, native libraries, resources, and shrinking rules.

Modules cannot inject arbitrary Android manifest XML or Gradle scripts. If the SDK needs integration that the declaration cannot express, add that capability to Ink's validated module manifest before publishing the package.

## Add a preview adapter

Implement the same contract with deterministic development data:

```ts
import battery from "../src/module";
import { definePreviewAdapter } from "@ink/native/preview";

export default definePreviewAdapter(battery, {
  scenarios: {
    normal: {
      status: () => ({ level: 0.72, charging: false, lowPowerMode: false }),
    },
    charging: {
      status: () => ({ level: 0.48, charging: true, lowPowerMode: false }),
    },
  },
});
```

Preview code runs on the development host and is not included in the APK. Include unavailable and permission-denied scenarios when the production capability can enter those states.

## Package native artefacts

Add the generated manifest and preview entry to `package.json`:

```json
{
  "name": "@example/ink-battery",
  "version": "1.0.0",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "ink": "./src/index.ts"
    }
  },
  "ink": {
    "module": "./dist/ink-module.json",
    "preview": "./dist/preview.js"
  },
  "dependencies": {
    "@ink/native": "^1.0.0"
  },
  "peerDependencies": {
    "ink": "*"
  }
}
```

The generated manifest contains hashes for its contract and native artefacts. It also declares the compatible native extension API range.

## Inspect and publish the module

Run these checks before publishing:

```sh
ink module build
ink module verify
ink module inspect .
npm publish
```

`ink module inspect` displays the public operations, permissions, components, dependencies, native size, ABI requirements, and preview scenarios. `ink module verify` checks generated hashes and rejects undeclared build hooks.

Follow semantic versioning for the app-facing API. Removing an operation or error tag, renaming a field, or changing a field type requires a major version.

## Keep modules focused

Prefer a package that gives apps a complete domain operation over a wide wrapper around one SDK. A maps module should own map camera and annotation behaviour, but it should not also acquire device location, persist favourites, or make routing API requests unless those features are inseparable from its interface.

Read [Build an Open-Meteo module](open-meteo-module.md) for a complete source-module example.
