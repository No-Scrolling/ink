# ADR 0002: Installed source packages

## Context

Ink applications need to share screens and complete flows without copying files between apps. The compiler previously followed only relative files inside one application. Reusing Node's JavaScript runtime or a bundler would undermine Ink's ahead-of-time Rust output, while inventing a separate registry and manifest format would duplicate package-manager behaviour.

Native Android integrations have a different trust and build model from declarative TSX source. Treating source lookup and native build activation as one mechanism would make the common package path harder to understand and maintain.

## Decision

An Ink source package is an ordinary installed package with public TSX entries in `package.json` `exports`. It uses an `ink` condition, with `types` before it when the same source also supplies TypeScript declarations. Applications add `ink` to `compilerOptions.customConditions` so TypeScript and Ink agree on the public entry.

`ink-compiler` owns one resolver adapter over Oxc Resolver. It accepts extensionless relative imports and bare package specifiers, selects only the `ink` condition, and compiles resolved `.tsx` screens through the existing restricted language. Resolved paths use their canonical identity for cycle detection. Relative imports cannot leave the importing package, and bare imports must resolve inside an installed package. URLs, absolute paths, queries, fragments, JavaScript and JSON are rejected.

No package source is evaluated. A package changes the APK only through the native Ink nodes and assets generated from the screens actually imported by the application.

Native extensions remain a separate framework interface. Oxc answers which source file an import names; extension metadata declares a capability and selects a framework-owned Android/Rust adapter from Ink's allow-list. It does not grant packages arbitrary compiler or Gradle execution. The first extension is `@ink/light-sdk`.

## Consequences

App authors use their existing package manager, lockfile and import syntax. Package exports provide a deliberate public surface, linked/workspace packages work through normal symlinks, and compiler changes remain invisible to application code.

The first source-package surface is intentionally screen-sized: one default, zero-argument component returning `Screen`. Reusable parameterised components need a deliberate typed interface. Native capabilities use the separate extension interface rather than being smuggled through source resolution.
