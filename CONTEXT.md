# Domain Context

## Purpose

Ink compiles a restricted TypeScript/TSX application into a small native Android application without shipping a JavaScript runtime.

## Core concepts

### State

A typed value declared by an Ink app and held in a native engine slot. Supported top-level values are booleans, integers, strings and homogeneous lists.

### Local state

State created with `state(initial)`. It belongs to one compile-time-expanded screen instance and returns to its initial value when the process restarts.

### Shared state

State created with `sharedState(key, initial)`. A stable key identifies one app-wide value for the life of the process. Declarations with the same key, lifetime, shape and initial value refer to the same native state slot.

### Persisted state

State created with `persistedState(key, initial)`. It has shared-state identity and is also saved in the Android application's private storage so it can survive process death and application upgrades.

### Hydration

Restoring compatible persisted values into their native state slots before Ink presents the first application frame. Missing, corrupt or incompatible values fall back to their compiled initial values.

### State key

An author-supplied, stable string literal used as the identity of shared or persisted state. Source order and generated numeric state IDs are not durable identities.

### State shape

The recursive compile-time type of a state value. Ink fingerprints the shape and validates restored values against it. Changing the shape resets that persisted key rather than attempting an implicit migration.

## Invariants

- Local state is never linked between screen instances or written to storage.
- One state key cannot mix shared and persisted lifetimes.
- Repeated declarations of one key have identical shapes and initial values.
- Hydration completes before surface attachment and the first visible frame.
- Storage failures never prevent the in-memory application from running.
- Navigation, focus and scroll position are engine mechanics, not application state, and are not persisted implicitly.
- Persisted state is for small settings and metadata, not resources, caches, media or secrets.

## Avoided terms

- Avoid **global state**; use **shared state** because its application scope and process lifetime are explicit.
- Avoid **storage state**; use **persisted state** for the application value and **persisted snapshot** for its encoded representation.
- Avoid **load state**; use **hydration** for the pre-frame restore operation.
