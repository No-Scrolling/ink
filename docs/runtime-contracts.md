# Native messages and resource ownership

Ink's foreground and background runtimes use JSON messages. The host accepts at most 256 queued messages in each direction, 256 native requests per runtime and 1 MiB per ordinary message. Development bundle evaluation has a separate bound.

## Message contract

| Message | Direction | Meaning |
| --- | --- | --- |
| `call` | JavaScript → host | `{id,module,operation,payload,timeoutMs,controller?}` registers one request before the adapter starts. |
| `result` | Host → JavaScript | `{id,value}` carries a string, or `{id,kind,message,retryable}` carries a failure. |
| `cancel` | JavaScript → host | `{id}` removes the request, cancels its timeout and asks its adapter to stop. Repeated cancellation is harmless. |
| `controller` | Host → JavaScript | `{id,value}` publishes state to an attached observer. |
| `commit` | JavaScript → foreground host | `{operations}` applies ordered renderer mutations. Workers cannot commit UI. |
| Shutdown | Host lifecycle | Close request registration, cancel outstanding native work, deactivate attached controllers, then drop the JavaScript runtime. |

Stable error names are `unavailable`, `permission-denied`, `permission-blocked`, `location-disabled`, `nfc-disabled`, `timeout`, `protocol`, `unexpected` and `busy`. Kotlin declares these strings explicitly; Rust uses the same names. Numeric JNI codes retain their existing values, with `busy` added as 8. An aborted JavaScript request rejects with the signal's reason, preserving ordinary AbortSignal behaviour.

`NativeRequests` owns registration, timeout, cancellation and exactly-once settlement for both Android hosts. It removes registration before delivering a result. Completion after cancellation, timeout or shutdown is discarded; temporary file results marked for deletion are deleted even when their owner has gone. An adapter still owns cancellation of its underlying work. Native request timeouts default to 30 seconds and are bounded to positive 32-bit milliseconds.

Request and controller identifiers come from one process-wide atomic allocator exposed only to the runtime. They are positive JavaScript-safe integers and are never reused by a replacement context. The foreground request registry also captures its session identity before dispatching a completion onto the UI thread. Controller events are accepted only for the current attached-controller map. Each worker has its own runtime handle and request registry. These checks prevent an old numeric completion from finding a new owner after reload.

## Controllers

Attaching registers the observer and starts activation. Commands wait for successful activation. Failed activation removes the JavaScript observer and the host's controller registration. Disposal removes the observer immediately, waits for pending activation to settle, and deactivates only after successful activation. Repeated disposal has no effect; commands after disposal reject. Session shutdown deactivates all controllers still registered in that session, without waiting for JavaScript effects to run. The current native activation implementations register their recipes synchronously; asynchronous commands remain cancellable requests.

Hooks release their own handles through effect cleanup. These rules do not change ordinary React effects or silently cancel arbitrary application promises. An explicit controller's owner must call `dispose`; stream/socket owners must close or cancel their handles.

## Ownership by package

| Package or facility | Owner and lifetime |
| --- | --- |
| `ink` renderer, navigation and controller observers | Foreground runtime; mounted hooks dispose their attached handles. Navigation hides retained screens with React Activity: state remains, while effects disconnect and Ink hooks release attached handles; revealing the screen reconnects effects and recreates those handles. |
| `@ink/audio` attached playback and `@ink/audio/capture` recorder and processors | Controller owner; pause follows the native adapter's app lifecycle. Disposal releases attached resources. |
| `@ink/audio` detached playback | Native media service; disposing a UI controller releases its connection and observer. Service playback survives UI disposal and can be reattached by session name. |
| `@ink/camera`, `@ink/camera/scan` | Mounted controller; camera presentation follows the visible portal and app pause/resume. Disposal closes its camera session. |
| `@ink/maps` | Mounted map; native rendering pauses when hidden or backgrounded. Disposal releases its view. Retained screens restore their last settled camera when effects reconnect. |
| `@ink/notifications` | Mounted subscription/controller; scheduled notifications have native lifetime after scheduling. |
| `@ink/location`, `@ink/nfc`, `@ink/lightos` | Native request or explicit subscription owner; request cancellation and host shutdown stop attached work. |
| `@ink/store`, `@ink/clipboard` | Individual calls; stored values outlive the call. Store change delivery belongs to the active UI context. |
| `ink` HTTP, streams and WebSocket facilities | Explicit request/reader/socket owner; native host shutdown stops foreground networking. |
| `@ink/background` | Durable scheduled job; each execution owns a separate runtime and supported non-UI adapters. UI disposal does not cancel scheduled jobs. |

## Backpressure and scheduling

A full JavaScript-to-native call queue rejects with retryable `NativeError("busy", …)`. The caller chooses whether to retry. Cancellation that cannot enter the queue still removes the JavaScript promise; the host's timeout remains the fallback for the outstanding native request.

A lost commit cannot be retried safely after the reconciler has advanced. Commit enqueue failure therefore marks the runtime as failed even if JavaScript catches the thrown transport exception. A terminal error is retried until the foreground host can receive it or shutdown interrupts delivery. Recovery replaces the UI runtime and tree. Native-to-JavaScript delivery failure must likewise be treated as runtime failure by the host, rather than pretending a result was delivered.

The runtime gives up to 256 promise jobs one turn, receives one native message, and services a due timer independently of whether reception timed out. Timers are ordered by deadline and identifier. Repeated promise chains, incoming messages and due timers therefore share turns; the interrupt flag and bounded job drain keep shutdown interruptible. This does not pre-empt a long synchronous native adapter call.

## Verification

See [verification results and remaining device checks](verification-2026-09-06.md).
