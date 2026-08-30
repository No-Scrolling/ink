# ADR 0001: Keyed shared and persisted state

- Status: Accepted
- Date: 2026-08-30

## Context

Ink screens currently declare local signals with `state(initial)`. Applications need settings and small metadata to be visible across otherwise independent screen modules and, in some cases, to survive process death. Ink has no JavaScript runtime, so provider trees, effect-driven hydration and JavaScript storage libraries would work against the framework's architecture and DX.

The current compiler expands screen modules independently and assigns positional state IDs. Those IDs change when source order changes and cannot act as shared or durable identity.

## Decision

Ink exposes three constructors with the same synchronous signal interface:

- `state(initial)` creates screen-instance-local state.
- `sharedState(key, initial)` creates app-wide, process-lifetime state.
- `persistedState(key, initial)` creates app-wide durable state.

Shared and persisted keys are required string literals. The compiler links repeated compatible declarations to one native state slot and rejects lifetime, shape or initial-value conflicts. Persisted records use the stable author key and a compiler-generated shape fingerprint rather than a generated state ID.

The native engine owns encoding, shape validation, hydration and dirty tracking. Android owns the app-private file path and atomic file replacement. Hydration is synchronous before the render surface is attached; writes are coalesced after interaction and flushed during lifecycle shutdown. Corrupt or incompatible data falls back safely to compiled defaults.

Version one deliberately provides no custom migration language. Renaming a key or changing its shape resets that value.

## Consequences

App authors gain shared state and persistence without providers, hooks, effects, promises or storage packages. Moving declarations between files does not lose persisted values as long as keys and shapes remain stable.

String keys must be maintained as part of the application's data model. Repeating a key in several screens also repeats its initial value, although the compiler verifies consistency. Imported state declarations may remove that repetition later when Ink gains general declaration modules as part of its package extension model.

Whole-store snapshots and a 1 MiB bound keep the codec and recovery model small. This is well suited to settings and app metadata but intentionally excludes large caches, resources and media. A future migration feature must be an explicit addition rather than hidden runtime coercion.
