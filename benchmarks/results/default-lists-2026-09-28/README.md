# Automatic native lists

The public `List` API now selects native row templates automatically when the compiler can preserve the row declaration. Existing app code does not need an opt-in or a new component. Rebuild with the updated Ink SDK.

Inline, fixed `Text`/`Stack` trees and standard `Row` declarations compile to templates. Rust owns mounted windows, keyed row identity and recycling. JavaScript supplies projected data and application callbacks. Custom React components, hooks, conditional trees, calls in rendered values, spreads and refs retain the React implementation. Indirect React text children fall back at runtime. The old implementation remains an internal compatibility path; it has not been deleted.

## CPU results

Mac headless operation `headless-20260928-162142-da0ed934`: 1,000 records, 20 warm-ups and 200 measured scroll steps per mode.

| Row implementation | Mean window update | p95 | JS window callbacks, including warm-up |
| --- | ---: | ---: | ---: |
| Compiled Text/Stack | 0.076 ms | 0.081 ms | 0 |
| Stateful React component | 0.604 ms | 0.684 ms | 220 |
| Compiled Row | 0.095 ms | 0.104 ms | 0 |
| Indirect React text content | 0.689 ms | 0.773 ms | 220 |

Hyperfine: five complete process runs, mean **0.394 seconds**. Native and React modes share text/layout geometry, but React includes a state hook and a benchmark-only window-settled acknowledgement. These are CPU scene measurements, excluding GPU, frame scheduling and physical display. They do not measure initial mounting or dataset replacement, and do not establish a universal eightfold improvement.

The native adapter projects the entire dataset when its data, render callback or key extractor changes and retains callback values per key. Scrolling avoids React work, but data refresh and memory costs require separate profiling. A change of template or between native and React rendering remounts the list.

## Checks

- Twelve compiler cases pass, covering eligible rows, aliases, shadowing and conservative fallbacks.
- Ink, the list fixture, template, Beeper and Reverb type checks pass.
- Rust native-list checks pass for bounded windows, keyed identity, event routing, boundaries and empty lists.
- Public List headless checks pass for row actions, local React state, indirect React content, bounded windows, prepend and native reverse anchoring.
- Existing explicit `view.list` regression workload passes, including controlled Japanese/emoji input across recycling and monotonic acknowledgements: `headless-20260928-162315-9df5e8ba`.
- Light Phone III emulator interactions exercised all four fixture modes, content/thumb scrolling, row actions, repeated state updates, prepend and reverse. Japanese text and emoji rendered.
- Existing template screens exercised a 5,000-row list, pagination beyond the initial 20 items, follow-to-end, clearing and repopulating.
- An isolated Reverb build opened its compiled album rows, its React track list and the playing screen; duration displayed 3:21. Playback was stopped afterwards.

## Screenshot validation correction

**The previously reported disappearing-text defect was a misinterpretation of the image presentation supplied to the assistant. It is not present in the saved PNGs.** The user correctly challenged the before/after comparison.

A direct pixel comparison of the 1080×450 header region found only 1,976 changed pixels (0.407%), bounded by x=278–386 and y=314–342: the row count changing from 1000 to 1001. The title, controls and selected-row label remain intact. OCR independently recognises every header control in both files. Evidence: `image-20260928-165908-7caa887f` and `image-20260928-165909-a94b9488`.

OCR also confirms intact headers in the formerly labelled `redraw-issue.png`, the repeated React-state update capture, and the follow-to-end capture (`image-20260928-165921-0303f8a3`, `image-20260928-165922-51000c03`, `image-20260928-165922-9c8e44d7`). The historical filename does not indicate an actual app defect.

The renderer experiments were unnecessary and have all been removed. No renderer workaround or graphics-backend change is required by this evidence. The emulator smoke checks above stand; they are not an exhaustive guarantee for every existing app. The failed visual diagnosis must not be used as evidence of an Ink rendering bug.

Final source APKs for the list fixture and template: `experiment-20260928-164842-e78e1181`. Reverb functional check: `experiment-20260928-160210-00f6f852`; its relevant list/compiler code predates the later indirect-text fallback, which does not affect its album rows. Candidate packages were isolated from existing apps and data. Build source archives and hashes remain in the agent-tool records; temporary build workspaces were removed.

## Files

- `crates/ink-compiler/src/native-lists.js`: conservative JSX conversion.
- `crates/ink-compiler/src/bundle-javascript.js` and `javascript.rs`: compiler integration.
- `packages/ink/src/list.ts` and `native-list.ts`: unchanged public API, projections, callbacks and React compatibility.
- `crates/ink-core/src/react/native_list.rs`: compiled row event routing and optional bound properties.
- `benchmarks/apps/ink-list` and `benchmarks/headless`: repeatable compatibility/scroll benchmark.

Run `scripts/agent-tools headless --app benchmarks/apps/ink-list --hyperfine --background` to repeat the CPU checks.
