# Runtime contracts

Run `bun run test runtime`. Use `--profile release` to run the same contracts with release Rust binaries. The runner always recompiles the selected fixture through `ink-compiler`, including its TypeScript check and production transforms. Generated output and fixture dependency links are ignored.

`test/native/tests/runtime.rs` runs the real `AppRuntime` and QuickJS. The bridge fixture imports `ink/native`; it does not replace the bridge, proxy detection or React renderer. Small raw JavaScript programmes exercise runtime transport and shutdown boundaries directly.

The contracts cover:

- Native request envelopes, pre-aborted requests, cancellation, late results and disposal of abort listeners after settlement.
- Native error fields, the 256-pending-request limit, and capacity recovery after completion.
- Outbound binary copy ownership, inbound JavaScript buffer lifetime and consumption exactly once.
- Per-message and per-binary size rejection, the 16 MiB shared queue budget, the 8 MiB outbound binary budget, and recovery or reclamation.
- Keyed latest-message coalescing without retaining superseded payloads.
- Terminal unhandled rejections, handled rejections, stopping an infinite microtask chain and closed delivery APIs.

The `split_web` target loads the actual optional compiler output through `AppRuntime::spawn_with_web_loader`. It checks that globals remain lazy, the asset loads once across URL/body/stream use, and a failed asset read permits a later retry. It also runs the genuinely compiled inline bundle in QuickJS without an optional asset loader. Both modes use independent URL, binary body, JSON and transformed stream expectations.

Startup gates use Rust channels to establish queue state deterministically. Event waits have a ten-second deadline; no arbitrary sleep establishes correctness. These are host correctness tests, not throughput or latency measurements. They do not cover Android module implementations, HTTP/socket IO or GPU rendering.
