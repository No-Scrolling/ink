---
title: "Native message path measurements"
description: "Isolated measurements and retention decisions for native message path candidates."
---

Wake coalescing and typed native results are retained. The search fast path is
dropped, and input/window protocol conversion is not implemented.

Measured on 7 November 2026 from `main` at
`7d9254f806a05da44b9ae7e794303ff69fe6456a`. Each candidate was compared
independently with saved main binaries and matching JavaScript assets.

## Method and noise bounds

Host measurements used an Apple M4 Pro, macOS 27.3 arm64, Rust 1.96.0 and
Bun 1.4.4. Builds completed before measurement. Five paired rounds alternated
before/after order: B/A, A/B, B/A, A/B, B/A. Tables report medians across all
samples. Negative latency changes indicate faster delivery.

A separate comparison ran the same main binaries on both sides. For each
workload, the noise bound is the largest absolute change between paired round
medians in that comparison. Retention requires a repeatable improvement beyond
the relevant bound. Allocation and wake counts use the same method.

Each implemented candidate ran the core suite with this saved-main comparison:

```sh
bun run bench:core --rounds 5 --samples 100 --warmup 20
bun run bench:core --rounds 5 --samples 100 --warmup 20 --baseline SAVED_MAIN_RUN
```

The first command saved main before production edits. The second command also
established core noise with unchanged main before comparing candidates. The
checked-in core and journeys benchmark sources did not change. The suite exercises real QuickJS, SDK fixtures and native
scene application. It excludes Android scheduling and display presentation.

Core control aggregate changes ranged from −1.79% to +4.48% for ordinary
scene workloads. Loaded input and sub-microsecond playback had larger
variation. The per-workload paired bound, rather than that aggregate range, is
used when assessing regressions. No general core speed-up is claimed.

## Search: dropped

The candidate replaced each guarded primitive-string `includes` call with a
UTF-16 substring match. It retained descriptor and intrinsic validation, the
JavaScript fallback and QuickJS lowercasing. UTF-16 conversion preserved unpaired
surrogates. The candidate added 42 lines and removed three; none are retained.

The unchanged journeys suite used five pairs, 20 samples and four warm-ups per
process. Every pair covered all 21 journeys, including independent ID/order,
native cache completion, native guard use and final scene contracts. Results
below cover `journey-local-search-cache-ready-*`.

| Workload | Before median, ms | After median, ms | Change | Noise bound |
| --- | ---: | ---: | ---: | ---: |
| 1,000 rows | 6.447 | 6.426 | −0.33% | 5.25% |
| 5,000 rows | 33.797 | 33.985 | +0.56% | 3.44% |
| 10,000 rows | 70.529 | 69.838 | −0.98% | 2.83% |

Changes did not beat noise and individual pairs changed direction. The separate
core comparison passed all 30 workloads and idle contracts. Apparent changes in
unrelated playback and CPU-load workloads do not support retaining this search
change.

## Emulator boundary

Android measurements used the existing `Light_Phone_III` API emulator,
`emulator-5554`, at its existing 1080 × 1240 resolution. The release probe used
only `com.vandam.ink.nativepath.t330c74009`. Foreground activity was checked
before each block, and logs were filtered to the probe process ID. No screenshots, other application operations, display or lock
changes, or accessibility configuration changes were needed.

The probe's public Ink press handler emitted 128 log events and updated its
rendered counter. Each sample required exactly 128 delivered events and a native
commit. Five pairs used 20 measured taps and four warm-ups per APK/process.
`ReactReady` instrumentation counted wake callbacks; `InputUp` to `ReactCommit`
measured input-up through native scene application. JNI wake instrumentation and
logging are included, so these are instrumented emulator timings, not LP3
hardware or touch-to-display measurements.

The identical-APK control measured 131 wakes per tap on both sides, including
bounded-drain continuations: a 0% count noise bound. Its input-to-commit medians
were 0.915 and 0.877 ms (−4.15% aggregate), with a 33.71% paired noise bound.
A physical phone was unavailable. Phone measurement is not needed to count
coalesced JNI calls; hardware latency and energy remain unmeasured.

## Wake coalescing: kept

A shared pending flag suppresses repeated callbacks while outgoing events remain
queued. When a nonblocking receive finds the queue empty, it clears the flag and
checks the queue again. An event racing with rearming is either received by that
check or schedules another wake. Blocking receives rearm before waiting. The
existing 64-event/4 ms drain limit and explicit continuation callback are unchanged.

| Workload or metric | Before median | After median | Change |
| --- | ---: | ---: | ---: |
| JNI wake callbacks per 128-event tap | 131 | 4 | −96.95% |
| Input-up to native commit, ms | 0.922 | 0.937 | +1.64% |

The count improvement exceeded the 0% control bound in all five pairs; paired
reductions ranged from 87.02% to 97.71%. All 25,600 authored log events and 200
measured commits were delivered across the comparison's measured taps. Warm-up
samples also required complete delivery. Bursts exceed the drain limit and
exercise its continuation. No latency improvement or regression is established
against the 33.71% latency bound. The isolated core comparison passed all 30
workloads and idle contracts; no aggregate regression exceeded its workload's
paired control bound. The decision retains fewer wake callbacks, not a claimed
phone latency or energy gain.

## Typed native results: kept

Rust queues the request ID, string value or error fields and invokes a private
SDK resolver. The public API still returns `Promise<string>`. Store and database
SDKs still parse their structured result string. Existing result listeners are
used, including replacement listeners. Embedders without the SDK resolver retain
`__inkReceive` delivery.

The same Serde JSON serializer counts the original envelope's encoded bytes
without retaining the envelope. Both the 1 MiB result limit and shared queue
byte charges retain that encoded size, including escaping. Cancellation,
request removal, deadlines and the 256-request limit retain their existing
paths. Error and timeout results also use typed delivery.

The host probe used real QuickJS, the public `callNative` and database SDK, and
`DataHost` with the embedded host SQLite backend. Five pairs used 100 samples
and 20 warm-ups in fresh processes. A store read returned a 900,000-character
JSON string plus entry metadata. The database query returned one 350,000-character
text column. Result lengths and authored endpoint characters were checked before
acknowledgement. Database creation and the initial durable store write were
outside timing. The store payload is close to 1 MB; SQLite's host backend has a
400,000-byte result cap, so a 1 MB query would measure rejection.

Latency spans host dispatch, the actual SDK request, worker execution, result
delivery and SDK parsing through the verified JavaScript acknowledgement.
Allocation instrumentation applies equally to both versions. Peak allocation
means additional live bytes tracked by Rust's global allocator during the call;
it excludes QuickJS/libc allocations, resident memory and total allocation churn.

| Workload or metric | Before median | After median | Change | Noise bound |
| --- | ---: | ---: | ---: | ---: |
| Store read, 900 KB, ms | 1.614 | 1.281 | −20.63% | 12.59% |
| SQLite query, 350 KB, ms | 0.606 | 0.472 | −22.16% | 2.33% |
| Store peak live Rust bytes | 4,501,520 | 2,701,352 | −39.99% | 0% |
| SQLite peak live Rust bytes | 1,751,489 | 1,050,947 | −40.00% | 0% |
| Missing-key store read, ms | 0.02177 | 0.02213 | +1.63% | 4.15% |
| Missing-key peak live Rust bytes | 1,370 | 772 | −43.65% | 0% |

Every paired large-store improvement was 18.97–24.18%, beyond its 12.59% bound.
Every paired large-SQLite improvement was 20.56–25.34%, beyond its 2.33% bound.
Allocation peaks were identical within each version across all paired rounds.
The isolated core comparison passed all 30 workloads and idle contracts, with no
aggregate regression above its paired workload bound. The typed-result build
also passed all 21 unchanged journeys, including continuous cancelled SQLite
search, route error/retry and persisted startup.

## Input and window messages: not implemented

The missing-key control did not establish a latency benefit for tiny payloads.
The large-result gain does not justify a broader input/window protocol conversion.
No input or window performance improvement is claimed, and no code from this
candidate is retained.

## Combined core comparison

All 30 workloads and idle contracts passed in all five alternating pairs, using
100 samples and 20 warm-ups per process. These are regression checks; no core
speed-up is claimed. Bounds are the maximum absolute paired control changes.

| Workload | Before median, ms | After median, ms | Change | Noise bound |
| --- | ---: | ---: | ---: | ---: |
| react-small | 0.908000 | 0.904792 | -0.35% | ±12.49% |
| react-bulk | 1.024334 | 1.024458 | 0.01% | ±9.10% |
| react-resize | 1.924042 | 1.946000 | 1.14% | ±10.11% |
| list-edit-100 | 0.131958 | 0.131584 | -0.28% | ±7.81% |
| list-scroll-100 | 0.013208 | 0.013583 | 2.84% | ±11.83% |
| list-edit-1000 | 0.184709 | 0.185084 | 0.20% | ±6.76% |
| list-scroll-1000 | 0.013541 | 0.013125 | -3.07% | ±7.18% |
| list-edit-10000 | 0.682167 | 0.671833 | -1.51% | ±10.11% |
| list-scroll-10000 | 0.014625 | 0.014417 | -1.42% | ±2.51% |
| scrollbar-drag-10000 | 0.074750 | 0.075416 | 0.89% | ±2.89% |
| rows-edit-100 | 0.166250 | 0.164375 | -1.13% | ±5.66% |
| rows-edit-1000 | 0.236333 | 0.237833 | 0.63% | ±2.22% |
| rows-prepend-1000 | 1.514791 | 1.516209 | 0.09% | ±6.96% |
| rows-reorder-1000 | 1.452291 | 1.460458 | 0.56% | ±11.61% |
| rows-native-scroll-1000 | 0.075708 | 0.074792 | -1.21% | ±1.93% |
| rows-react-scroll-1000 | 0.779917 | 0.789333 | 1.21% | ±6.96% |
| rows-pure-scroll-1000 | 0.108833 | 0.108959 | 0.12% | ±2.01% |
| rows-decoded-image-arrival | 0.002333 | 0.002458 | 5.36% | ±30.80% |
| startup-runtime-to-library | 3.333375 | 3.359209 | 0.78% | ±6.20% |
| navigation-open | 1.047166 | 1.042125 | -0.48% | ±14.23% |
| navigation-back | 0.285208 | 0.283959 | -0.44% | ±5.20% |
| navigation-tab | 0.201958 | 0.201458 | -0.25% | ±2.58% |
| conversation-controlled-input | 0.188167 | 0.189042 | 0.47% | ±4.83% |
| conversation-append | 0.563500 | 0.573541 | 1.78% | ±6.05% |
| conversation-prepend | 0.427709 | 0.436708 | 2.10% | ±6.00% |
| conversation-input-background | 0.643917 | 0.552542 | -14.19% | ±32.66% |
| conversation-input-under-load | 1.709500 | 1.702709 | -0.40% | ±12.33% |
| playback-native-clock | 0.000125 | 0.000125 | 0.00% | ±50.60% |
| playback-state-change | 0.095208 | 0.098375 | 3.33% | ±18.50% |
| conversation-input-cpu | 79.403250 | 72.448083 | -8.76% | ±12.71% |

## Verification and cleanup

The final combined build passed:

- `bun run check`.
- `bun run test runtime scene compiler sdk --profile release`: 29 runtime, 18
  scene, 52 compiler and 52 SDK tests, with no failures.
- `cargo test --release -p ink-runtime`: two binary transport tests, with no failures.
- The Android release bytecode smoke probe: exact equality after a public store
  write/read of 900,000 characters, plus a `NativeError` with the existing
  `protocol` kind for an unknown store operation. The initial probe incorrectly
  expected `unexpected`; correcting that probe required no production change.

No test files were added or changed. Existing tests cover request cancellation,
late results, pending-request capacity, binary ownership, store size rejection,
SQLite results and controlled-input ordering. No rendering code changed.

The probe package was uninstalled. Comparing `pm list packages` immediately
before and after uninstall showed exactly one removal,
`com.vandam.ink.nativepath.t330c74009`. Comparing with the initial package list
showed no other removals. Accessibility remained `null` and the display remained
1080 × 1240. No other apps or device display/lock settings were changed.

Production commits:

- `7a8e993612cafdd044bc5f146ae3d0bed9918e67`: wake coalescing.
- `8bbb34445bb2a2155c249bd828fc5f5daa25a100`: typed native results.

Raw samples, saved executables, probe APKs and build outputs were temporary and
were removed after the summaries were recorded. No raw evidence is committed.
Measurements establish host large-result latency and Rust allocation reductions,
and emulator JNI callback reductions. They do not establish LP3 hardware
latency, whole-process allocation peaks, battery use or display presentation.
