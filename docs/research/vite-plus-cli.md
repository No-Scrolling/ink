# Vite+ CLI research for Ink

Research date: 29 August 2026. Source snapshot: local `~/Developer/vite-plus` at commit [`3380eb8`](https://github.com/voidzero-dev/vite-plus/tree/3380eb8692c4b60fa7e90aefa8c077b5e7d24163).

## Short answer

The pleasant `vp` experience is primarily a standalone Rust CLI built with `clap`, with deliberately custom help and output on top. Vite+ is actually a hybrid: its global `vp` launcher is Rust; its project package contains TypeScript commands and a Rust NAPI binding; the global launcher resolves and delegates to that local package where appropriate. This split exists because Vite+ orchestrates a JavaScript toolchain and manages Node.js itself. Ink does not need that distribution complexity.

For Ink, the useful model is one small Rust `ink` binary using `clap` derive. It should call compiler/build modules directly and own project discovery, subprocesses, diagnostics and device interaction. Do not copy Vite+'s Rust/TypeScript/NAPI global/local split.

## What Vite+ uses

- `crates/vp_global_cli` compiles the `vp` binary and depends on `clap` with derive, Tokio, `owo-colors`, `indicatif`, `dialoguer` and `crossterm` ([Cargo manifest](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/Cargo.toml#L10-L49)). The workspace currently pins `clap` 4.5.40 ([workspace manifest](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/Cargo.toml#L173-L187)).
- Its top-level command tree is ordinary `#[derive(Parser)]` and `#[derive(Subcommand)]` Rust. Commands that another layer owns accept trailing arguments unchanged; native commands use typed fields ([CLI definitions](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/cli.rs#L41-L209)).
- `packages/cli/src/bin.ts` is the local JavaScript entry point. It handles a small set of TypeScript commands and sends the rest through the Rust NAPI binding ([local dispatch](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/cli/src/bin.ts#L1-L10), [routing](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/cli/src/bin.ts#L110-L156)). Even TypeScript-backed commands such as `create` now define and parse their arguments with Rust `clap` through NAPI, keeping argument semantics and help presentation consistent ([create arguments](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/cli/binding/src/js_command_args/commands/create.rs#L17-L153), [NAPI parse result](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/cli/binding/src/js_command_args/commands/create.rs#L195-L227)).
- The NAPI binding is a Rust `cdylib`; the package build bundles TypeScript and separately compiles platform-specific native bindings ([binding manifest](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/cli/binding/Cargo.toml#L13-L54), [build architecture](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/cli/BUNDLING.md#L5-L57)). This is justified for Vite+ but would add no leverage to Ink's all-Rust implementation.

## Why `vp` feels good

### A compact, predictable command model

`vp` reserves short, task-shaped commands (`dev`, `build`, `check`) and keeps global options scarce. It supports `-C <DIR>` so commands can target a project without a shell `cd`, rewrites a few intentional aliases, and treats `vp help build` like `vp build --help` ([argument normalisation](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/main.rs#L78-L153), [`-C` definition](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/cli.rs#L41-L62)).

The top-level help is not raw generated `clap` output. Vite+ describes commands in purposeful sections such as Start, Develop, Execute, Build and Maintain, then injects that document into a custom `clap` help template ([help document](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/help.rs#L194-L295), [template integration](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/cli.rs#L1168-L1208)). Its shared renderer aligns rows, wraps to terminal width and disables styling for non-TTY, `NO_COLOR`, `CLICOLOR=0` and `TERM=dumb` contexts ([help rendering](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_cli_help/src/lib.rs#L125-L169), [row wrapping](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_cli_help/src/lib.rs#L231-L312)).

### Calm, consistent output

The shared Rust output module defines a small vocabulary: `info:`, `pass:`, `warn:`, `error:`, `note:` and a green success check. Warnings/errors/notes go to stderr, while result output normally goes to stdout; shim mode can route human output away from stdout so wrapped tools remain pipeable ([output module](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_shared/src/output.rs#L1-L113)).

Long operations use `indicatif`, but only show animated progress on an interactive stderr outside CI. Otherwise the progress bar is hidden, leaving stable logs, followed by explicit success lines ([global install progress](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/commands/global/install.rs#L200-L221), [success output](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/commands/global/install.rs#L530-L541)).

Unknown commands are promoted from raw parser failures into a concise branded error. A close top-level typo can be suggested and, only in an interactive terminal, offered for execution; non-interactive use remains deterministic ([suggestion handling](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/main.rs#L155-L228), [parse-error routing](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/main.rs#L481-L531)). Running bare `vp` in a TTY opens a searchable command picker built with `crossterm`; non-interactive invocations skip it ([picker](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/crates/vp_global_cli/src/command_picker.rs#L1-L142)).

## Installation, distribution and local development

The released command is a standalone Rust launcher. Vite+'s design goal is that it can be installed without an existing Node.js runtime, then provision Node internally when JavaScript work is required ([implemented design RFC](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/rfcs/global-cli-rust-binary.md)). Platform-specific native bindings are published separately so package managers select only the host package ([platform packages](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/cli/BUNDLING.md#L262-L277)).

For repository development, `pnpm bootstrap-cli` builds the packages and release Rust launcher, then runs its installer ([root script](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/package.json#L6-L10)). The installer creates a timestamped local version, copies the built launcher into it, points `current` at the active version and exposes `vp` through a stable user-bin symlink ([development installer](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/tools/src/install-global-cli.ts#L66-L202), [Unix links](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/cli/install.sh#L1289-L1404)). Its JavaScript package is symlinked to source for local work, but the Rust launcher is copied, so Rust launcher changes still require another bootstrap ([local dependency links](https://github.com/voidzero-dev/vite-plus/blob/3380eb8692c4b60fa7e90aefa8c077b5e7d24163/packages/tools/src/install-global-cli.ts#L393-L433)).

That production-grade version indirection is unnecessary for Ink today. To satisfy Ink's stronger local requirement—that changing CLI source updates the next `ink` invocation—the shell entry should be a stable development shim that runs the workspace package through Cargo. Cargo's incremental build then recompiles only when sources changed. A copied `cargo install --path` binary would not provide this property.

## Recommendation for Ink v0

Adopt these ideas:

- Add a dedicated `ink-cli` binary crate using `clap` derive; keep compilation as a library call rather than spawning the current compiler executable.
- Keep the initial interface deliberately small: `ink check`, `ink build`, `ink dev` and `ink doctor`, plus `-C <DIR>`, `--verbose` and `--version` where useful.
- Discover `ink.toml` from the working directory and keep compiled app artefacts, Gradle paths and ADB details behind the CLI interface.
- Build one small output module with TTY-aware colour, plain CI output, diagnostic text on stderr and machine-meaningful results on stdout.
- Show quiet phase lines or a spinner for slow builds, clear it before streaming an underlying failure, and expose full Cargo/Gradle/ADB output through `--verbose`.
- Write intentional, grouped help instead of accepting `clap`'s default layout. Ink only has a few commands, so this should be tens of lines rather than a reusable rendering framework.
- Install a stable `ink` development shim into a directory already on the user's `PATH`. Have it invoke `cargo run --manifest-path <Ink>/Cargo.toml -p ink-cli --quiet -- "$@"`, so source changes are rebuilt automatically on the next command.

Do not adopt these yet:

- Vite+'s global-versus-local CLI, managed runtime, NAPI or platform-package machinery.
- A no-argument interactive picker, typo execution prompts or self-upgrade. They are polished, but they deepen an interface that has not settled.
- A decorative terminal header on every command. Ink's useful identity can come from concise wording and one accent colour; routine `dev` rebuilds should stay visually quiet.

The central lesson is that `vp` is pleasant less because of a particular UI crate and more because its command grammar, help, output streams, TTY behaviour and installation path are treated as one coherent interface. Ink can obtain most of that feel with `clap`, `owo-colors` and optionally `indicatif`, without inheriting Vite+'s much larger orchestration architecture.
