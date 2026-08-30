# ADR 0004: Async resources and native actions

## Context

Ink applications are synchronous descriptions compiled to Rust, while LightOS, network and device capabilities complete asynchronously on Android. Exposing JNI, Binder or generic native command strings to application code would make those implementation details part of Ink's public interface.

## Decision

Native modules expose typed constructors such as `lightSdkVersion()`. A constructor returns a discriminated `AsyncResource<T>`: loading has only `status`, ready has `value`, and error has a structured `ResourceError`. Errors have a stable kind, a message and an explicit retryable flag. TypeScript narrows the available fields from the status check.

The compiler gives every resource a dedicated `ResourceId` and attaches resources declared by screen modules to that screen. `ink-core` activates only resources owned by the visible route and tab, plus any application-owned resources. Leaving a screen cancels its active work. Returning uses a settled cached value; an explicit reload starts a fresh read.

Each resource has at most one request in flight. Reloading cancels the previous request before starting the replacement, and late completions are ignored. Every operation carries a framework-defined timeout. Android cancels timed-out adapter work and returns a typed timeout error to Rust.

Native requests are tagged as resource reads, one-shot actions or cancellations. Reads populate resource state; actions report completion without becoming cached state. Android drains the request queue and dispatches each operation to an allow-listed native-module adapter. Adapters may do background work, but completion re-enters Rust from Android's UI thread. The JNI seam carries requests and tagged results; it does not contain module-specific Binder behaviour.

Permission access is the first paired capability. `lightSdkPermission("camera")` reads the LightOS permission state and exposes `request()` as a native action. The action completes after LightOS presents its permission activity rather than waiting on the user. When the Ink activity resumes, active resources reload and reflect the decision. The compiler includes the Android camera permission only when that resource is used.

## Consequences

Application code stays declarative and cannot issue arbitrary native calls. New capabilities require a TypeScript declaration, compiler lowering and an Ink-owned native adapter, which is more deliberate but keeps permissions, lifecycle and compatibility review inside the framework. The runtime borrows Effect's useful semantics—typed failures, scoped lifetime and interruption—without adding Effect or a JavaScript runtime to applications. Automatic retries remain a module-level decision because only a capability knows whether an operation is safe to repeat.
