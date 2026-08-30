# Oxc package resolution for Ink

Research date: 30 August 2026. Sources are limited to Oxc Resolver's official documentation and source, Node.js documentation and the TypeScript handbook.

## Recommendation

Use `oxc_resolver` beneath Ink's existing source graph, but keep the package contract narrower than general Node.js resolution. An Ink source package should expose TypeScript or TSX through the standard `package.json` `exports` field under an `ink` condition:

```json
{
  "name": "@ink/weather",
  "exports": {
    ".": {
      "types": "./index.d.ts",
      "ink": "./src/index.tsx",
      "default": "./dist/index.js"
    }
  }
}
```

Node recommends `exports` for new packages because it defines public entry points, supports conditions and encapsulates unexported subpaths; when both `exports` and `main` exist, `exports` takes precedence ([Node package entry points](https://nodejs.org/api/packages.html#package-entry-points)). Custom conditions are a standard part of conditional exports, package key order is significant, and `default` is the universal fallback ([Node conditional exports](https://nodejs.org/api/packages.html#conditional-exports), [community conditions](https://nodejs.org/api/packages.html#community-conditions-definitions)). Putting `types` first keeps the package conventional for TypeScript, while the compiler selects `ink` source without treating Ink as Node or a browser.

Oxc should only locate source files. Ink should continue to parse and validate every resolved file against its restricted language, build its own module graph and lower that graph to Ink IR. No package JavaScript is executed and no JavaScript runtime enters the APK.

## Rust interface

At Oxc Resolver 11.24.3, `Resolver` is the operating-system-filesystem alias for `ResolverGeneric<FileSystemOs>`. It is constructed with `Resolver::new(ResolveOptions)` and caches filesystem/package reads internally. `resolve_file(importer, specifier)` accepts the absolute path of the importing file and returns `Result<Resolution, ResolveError>`; unlike the directory-based `resolve`, it also supports automatic tsconfig discovery ([constructor and cache](https://docs.rs/oxc_resolver/11.24.3/oxc_resolver/struct.ResolverGeneric.html#method.new), [file-aware resolution](https://docs.rs/oxc_resolver/11.24.3/oxc_resolver/struct.ResolverGeneric.html#method.resolve_file), [official source](https://github.com/oxc-project/oxc-resolver/blob/b6a6125c5556ca1ae0e48b479d203e8f06effd10/src/lib.rs#L225-L271)). Ink should use the file-aware call because it naturally follows each import from its containing module:

```rust
let resolver = Resolver::new(ResolveOptions {
    condition_names: vec!["ink".into()],
    extensions: vec![".tsx".into(), ".ts".into()],
    node_path: false,
    builtin_modules: true,
    ..ResolveOptions::default()
});

let resolution = resolver.resolve_file(importer, specifier)?;
let source_path = resolution.into_path_buf();
```

`Resolution::path()` and `into_path_buf()` return the resolved filesystem path without a query or fragment. It can also expose the associated `package.json`; module type is only populated when that option is enabled ([resolution API](https://docs.rs/oxc_resolver/11.24.3/oxc_resolver/struct.Resolution.html)). Ink should reject query/fragment-bearing imports rather than silently discarding them because they are bundler conventions with no Ink semantics.

## Options for the first implementation

| Option | Ink value | Reason |
| --- | --- | --- |
| `condition_names` | `["ink"]` | Select the framework-specific source branch. Oxc always considers a `default` branch and otherwise matches configured conditions in package key order ([Oxc condition matching](https://github.com/oxc-project/oxc-resolver/blob/b6a6125c5556ca1ae0e48b479d203e8f06effd10/src/lib.rs#L1774-L1791)). Do not claim the `node` or `browser` environments. |
| `exports_fields` | default `[["exports"]]` | Preserve the standard package boundary rather than inventing an Ink-specific top-level entry field. |
| `extensions` | `[".tsx", ".ts"]` | Ink consumes source, not JavaScript, JSON or native modules. The list is tried in order for extensionless requests ([Oxc options](https://github.com/oxc-project/oxc-resolver/blob/b6a6125c5556ca1ae0e48b479d203e8f06effd10/src/options.rs#L80-L93)). |
| `extension_alias` | empty initially | Do not make `./thing.js` silently mean `./thing.ts` until real packages need TypeScript's substitution convention. Exact `.ts`/`.tsx` export targets already resolve. |
| `main_fields` / `main_files` | defaults | Retain the resolver's `main` and `index` fallback for packages without `exports`, but accept the result only if it is supported Ink source. Prefer and document `exports` for Ink packages. Oxc defaults these to `main` and `index` ([defaults](https://github.com/oxc-project/oxc-resolver/blob/b6a6125c5556ca1ae0e48b479d203e8f06effd10/src/options.rs#L546-L576)). |
| `modules` | default `["node_modules"]` | Supports ordinary npm-compatible installs, nested dependencies and scoped packages. |
| `node_path` | `false` | Oxc otherwise appends absolute directories from the deprecated `NODE_PATH` environment variable. Disabling it makes resolution depend on the app's installed graph rather than machine-global state ([option definition](https://github.com/oxc-project/oxc-resolver/blob/b6a6125c5556ca1ae0e48b479d203e8f06effd10/src/options.rs#L117-L123), [NODE_PATH switch](https://github.com/oxc-project/oxc-resolver/blob/b6a6125c5556ca1ae0e48b479d203e8f06effd10/src/options.rs#L173-L182)). |
| `symlinks` | default `true` | Resolve workspace and linked packages to their real paths, matching Node's realpath-oriented module behaviour. Oxc warns that this can change `npm link` dependency lookup; that should surface as a normal resolution error rather than creating two identities for one module ([Oxc symlink option](https://github.com/oxc-project/oxc-resolver/blob/b6a6125c5556ca1ae0e48b479d203e8f06effd10/src/options.rs#L151-L171), [Node symlink behaviour](https://nodejs.org/api/modules.html#package-manager-tips)). |
| `builtin_modules` | `true` | Produce a specific builtin-module error for imports such as `node:fs` or `fs`; native Node APIs cannot run in an Ink package. |
| `tsconfig` | `None` initially | Installed package lookup does not require aliases, and one fewer project-level rule keeps compiler and editor resolution easy to understand. |

Oxc supports manual or automatic tsconfig discovery, project references and `compilerOptions.paths`; automatic discovery only works with `resolve_file` ([tsconfig options](https://docs.rs/oxc_resolver/11.24.3/oxc_resolver/struct.TsconfigOptions.html), [resolver discovery contract](https://docs.rs/oxc_resolver/11.24.3/oxc_resolver/struct.ResolverGeneric.html#method.resolve_file)). TypeScript explicitly notes that `paths` does not rewrite emitted imports: another runtime or bundler must implement the same mapping ([TypeScript `paths`](https://www.typescriptlang.org/tsconfig/paths.html)). Ink can opt into `TsconfigDiscovery::Auto` later if apps demonstrate a need, but package support should not introduce aliases by accident.

## Resolution and graph policy

Classify an import before calling Oxc:

1. Keep `ink` as a compiler-known module.
2. Resolve relative specifiers and bare package specifiers with `resolve_file`.
3. Reject absolute filesystem paths, URLs, query strings and fragments as unsupported syntax.
4. Accept only regular `.ts` and `.tsx` files; reject declarations, JavaScript, JSON and native modules even if Oxc can locate them.
5. Canonicalise resolved paths and key the source graph by canonical path. Track visiting and visited modules so cycles receive one clear Ink diagnostic and symlinked aliases do not compile twice.
6. Respect `exports` failures. Never recover from `PackagePathNotExported` by manually probing a package's filesystem: `exports` deliberately defines its public surface ([Node exports encapsulation](https://nodejs.org/api/packages.html#main-entry-point-export)).

Oxc's non-exhaustive `ResolveError` distinguishes not-found modules, bad aliases, malformed package/tsconfig JSON, invalid specifiers, builtins, invalid exports targets and unexported package paths ([official error source](https://github.com/oxc-project/oxc-resolver/blob/b6a6125c5556ca1ae0e48b479d203e8f06effd10/src/error.rs#L10-L126)). Ink should attach the error's display text to the original import span and add framework-specific help only for common cases such as an uninstalled package or a missing `ink` export.

## Containment

Node's export-target rules already require targets to start with `./` and forbid traversal, `node_modules` segments and targets outside the package root ([Node export target validation](https://nodejs.org/api/packages.html#path-rules-and-validation-for-export-targets)). That protects the external entry point, but it does not constrain relative imports inside the selected source file.

Ink should therefore apply an ownership check after resolution rather than a single resolver `restriction` rooted at the app. A linked workspace package legitimately resolves outside the app directory when symlinks are followed. For each graph root, record either the canonical app root or the canonical directory containing the resolved package's `package.json`; relative imports must remain within that owner. Bare imports may enter another installed package through its public exports and establish a new owner. This permits workspace packages and declared dependencies while preventing a package source file from importing arbitrary files elsewhere on the developer's machine.

Package resolution is not a sandbox. Installing an npm package may itself invoke package-manager lifecycle scripts, and a future native extension deliberately adds build-time/native capabilities. Ink's guarantee here is narrower: its compiler never evaluates resolved TypeScript/JavaScript and only reads source files inside the validated module graph.

## Implementation sequence

1. Add `oxc_resolver` to `ink-compiler` and replace handwritten relative `.tsx` probing with one resolver-backed source loader.
2. Preserve the existing relative-screen behaviour, then add one fixture package exposing an `ink` conditional export.
3. Centralise import classification, supported-extension checks, canonical module identity and import-span diagnostics in that loader.
4. Add package ownership/containment and cycle detection before expanding the supported package grammar.
5. Keep native extension metadata separate from source resolution. Oxc answers which source file an import names; Ink's package model decides which declared Rust/Android capabilities that package activates.
