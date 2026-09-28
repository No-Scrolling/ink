# Direct native update diagnostic

JavaScript app logic can update Ink's existing native interface substantially faster when a fixed screen sends changed values directly. This experiment changes the update model; it is not a faster React implementation or completion of the React compatibility goal.

## Paired results

Both variants use the **same binary**, QuickJS, native commit transport, Rust layout and native scene checks. The direct variant replaces only the JavaScript bundle with [a small fixed-screen diagnostic](diagnostic.js). Order is React/direct/direct/React repeated three times, with 20 warm-up updates per process. No samples were discarded.

| CPU input to scene | React mean | Direct mean | React p95 | Direct p95 |
| --- | ---: | ---: | ---: | ---: |
| Mac, 1,200 updates per variant | 1.608 ms | 0.278 ms | 1.765 ms | 0.309 ms |
| Physical LP3, 600 updates per variant | 7.251 ms | 2.945 ms | 7.522 ms | 3.327 ms |

This is a 5.79× Mac and 2.46× LP3 reduction in mean CPU update duration. The phone harness runs as Android shell, continuously, without an Activity. It excludes Android input delivery, app scheduling, GPU submission and display latency. It does not establish a sub-16 ms full-app p95.

| Mean stage | Mac React | Mac direct | LP3 React | LP3 direct |
| --- | ---: | ---: | ---: | ---: |
| Input | 0.0005 ms | 0.0004 ms | 0.005 ms | 0.008 ms |
| JavaScript and transport | 1.511 ms | 0.188 ms | 6.650 ms | 1.917 ms |
| Native apply and layout | 0.097 ms | 0.090 ms | 0.595 ms | 1.020 ms |

The phone's native phase is slower in the shorter direct workload despite using identical native code. CPU frequency and scheduling were not traced for this comparison, so its cause is unproven. These are measured wall-clock results, not a prediction at fixed clocks.

## What the diagnostic preserves

- JavaScript event handling, state, 500 label calculations, row grouping and alternating row gaps.
- All 500 mounted cells, including cells outside the viewport. Every measured update changes all 500 labels and the 50 parent row gaps.
- The same native controls, font settings, transport, layout and scrolling implementation.
- Every existing headless check: labels on every update, row gaps, thumb dragging, content dragging, reverse, hide/show, and count/scrollbar changes.

The diagnostic explicitly stores native node IDs and emits updates directly. It has no React reconciliation, hooks, effects, concurrent rendering, Activity lifecycle or component-local state. Reverse changes displayed values in place; it does not demonstrate keyed component-state preservation. Count changes rebuild the body. Passing this fixture is not proof of general React equivalence or template compatibility.

## Architectural implication

Ink already lays out and draws its interface in Rust. The opportunity is to simplify the layer that decides which native properties need changing:

```text
JavaScript action and app state
          ↓
changed bindings in one atomic batch
          ↓
retained Rust controls → affected layout and drawing
```

For a counter, JavaScript increments its state and updates a text binding. For the tuner, JavaScript can keep permissions and musical calculations while a binding updates the native indicator. For Podcasts, feed retrieval, playback decisions and downloads remain JavaScript; native controls can own their visual state. PlayingProgress already interpolates its progress in Rust, so moving that interpolation is not a new optimisation.

GPUI is a useful ownership reference: its App owns entity data, entities can notify observers, and windows hold renderable root entities ([official contexts documentation](https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md)). The proposed Ink direction is not an implementation of GPUI or a claim that GPUI's render model is identical to direct bindings.

A general API would still need stable list keys, conditional view lifetimes, subscription disposal, fresh callbacks and atomic updates. Keep the React adapter for existing applications; prove a small binding API on real counter, tuner and playing screens before considering migration. JSX syntax could remain, with a different runtime/compiler underneath. No production API was changed by this diagnostic, and no APK-size saving was measured.

## Evidence and reproduction

- `mac-paired.json.gz` and `phone-paired.json.gz` contain every measured sample, stage summaries, native binary provenance and both asset hashes. The harness's generic note mentions React; the direct variant explicitly does not contain React.
- Mac binary: `.agent-tools/headless-20260928-125646-3d8e6b96/ink-headless`.
- Android binary: `.agent-tools/headless-20260928-125845-516528a9/ink-headless`.
- The original React assets are in each prepared run's `assets` directory.
- To reproduce the direct Mac run, put `diagnostic.js` in a separate assets directory as `app.js`, copy `ink-icons-v1.bin` from the prepared run, then run the same binary with `--assets DIRECTORY --iterations 200 --warmup 20`. Alternate the assets directory for the React control.
- The ignored orchestration helper is `.agent-tools/direct-ui/compare.py`; phone execution used the reserved device API and removed its own remote scratch directory afterwards.
