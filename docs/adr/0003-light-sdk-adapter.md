# ADR 0003: Light SDK adapter

## Context

Ink apps need LightOS capabilities, but the upstream Android client owns Compose UI, navigation and application conventions that Ink already replaces. Depending on that client would increase application size and split ownership of lifecycle and interface behaviour. Copying protocol calls into every app would instead make each app responsible for upstream changes.

## Decision

Ink owns one allow-listed `@ink/light-sdk` extension. A side-effect import activates a conditional Android adapter; it never executes package JavaScript. The package manifest pins Light SDK 0.1.1 and records upstream commit `3df3c24a21247e70ad59e1bc0393ac6d63840bc2`.

The adapter implements the smallest useful headless protocol slice: LightOS marker registration, explicit service binding, token authentication, version checking and haptic-preference synchronisation. It has no dependency on Compose or the upstream client module. The physical device server is the default, while `ink.toml` may select the official emulator server during development.

Further capabilities will enter through small Ink-owned interfaces. Updating upstream compatibility is framework work: review the pinned Light SDK changes, update and verify this adapter, then release a new Ink package version. Applications receive compatible improvements by updating Ink rather than changing their own integrations.

## Consequences

Applications opt in with one import and apps that do not opt in carry no LightOS client code. Ink owns the compatibility burden and can keep its public interface stable even if Light SDK internals change. Supporting a new upstream release is deliberate rather than automatic, and a mismatched package or connected server produces a clear version diagnostic.
