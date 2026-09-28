# LP3 validation — 28 September 2026

The overnight candidate improves the unchanged 500-cell + resize workload, but **does not meet the physical-phone p95 goal** with ADB-generated taps. Mean native pointer-up to matching frame submission falls from **18.11 to 15.29 ms** (15.6%); p95 falls from **31.06 to 27.33 ms**. Mac headless acceptance remains 1.4983 ms mean / 1.5646 ms p95. No source changes were made during this comparison.

## Paired comparison

Candidate `experiment-20260928-043239-d135372a`; baseline `experiment-20260927-234629-c75ae781`. Physical device `LP3LHMA531900140`, Android API 34, 1080×1240. Candidate/baseline/baseline/candidate installation order. Each installation had one predefined warm-up probe and two measured three-second probes. All twelve probes, including warm-ups, are preserved in `comparison.json.gz`. Each probe accepted all 15 updates with zero rejected samples; screenshots confirmed 500 cells, Resize on and the expected 15/30/45 update counts. Foreground, unchanged-process and thermal guards passed. No builds ran during measurement.

| Pointer-up to frame submission | Baseline | Candidate |
| --- | ---: | ---: |
| Measured samples | 60 | 60 |
| Mean | 18.111 ms | 15.294 ms |
| Median | 13.856 ms | 12.253 ms |
| Nearest-rank p95 | 31.056 ms | 27.326 ms |
| Maximum | 32.158 ms | 28.043 ms |

| Stage — arithmetic mean | Baseline | Candidate |
| --- | ---: | ---: |
| Input dispatch | 0.327 ms | 0.317 ms |
| JavaScript, React and transport | 12.688 ms | 11.593 ms |
| Queue to native apply | 0.056 ms | 0.054 ms |
| Native apply/layout | 2.400 ms | 0.836 ms |
| Commit to frame submission | 2.639 ms | 2.494 ms |
| **Total** | **18.111 ms** | **15.294 ms** |

Native apply/layout mean fell 65.2%. Stage means add to the total; stage percentiles would not. This interval excludes physical touch sensing and panel response. Both APKs include the same bridge instrumentation.

| Measured run | Median | p95 / maximum |
| --- | ---: | ---: |
| Candidate 0/1 | 12.089 ms | 23.706 ms |
| Candidate 0/2 | 14.588 ms | 28.043 ms |
| Baseline 1/1 | 13.501 ms | 29.601 ms |
| Baseline 1/2 | 22.933 ms | 32.158 ms |
| Baseline 2/1 | 13.961 ms | 31.692 ms |
| Baseline 2/2 | 13.784 ms | 30.561 ms |
| Candidate 3/1 | 11.965 ms | 25.765 ms |
| Candidate 3/2 | 12.314 ms | 27.947 ms |

## Slow-update investigation

A separate Perfetto run (`probe-20260928-110407-623a82c9`, excluded from the paired comparison) recorded scheduler slices and CPU frequencies. In its first three updates JavaScript/transport took 19.29–21.83 ms, with 18.47–19.92 ms on CPU at an average 695 MHz. The remaining twelve took 8.54–9.94 ms, averaging roughly 1.92–2.08 GHz while running. Almost all execution was on the preferred performance cores, 6 and 7. `final-js-cpu.json` preserves each sample's CPU time, frequencies and cores.

This points to clock-speed variation as the main reason for the long tail in this trace; it does not prove that all variability comes from clocks. The process and JavaScript thread were already in Android's `/top-app` cpuset. Battery saver was off. The phone reports Performance Hint HAL support as false, so adding that API would not fix this device. No governor, frequency limit or battery setting was changed for these results.

The user then supplied 15 real refresh taps, recorded after `manual-lp3-final-ready` with no injected taps in that window. All 15 were accepted, with zero rejected samples. Median was 26.063 ms and p95 27.092 ms (`manual-result.json`, `manual-ready.log`). These observations do not support attributing the slow tail solely to ADB injection. They are a separate diagnostic sample, not pooled into the paired comparison. The p95 goal remains incomplete; the Mac result cannot substitute for it.

## Physical template compatibility

The isolated final template package `com.vandam.benchmark.ink.goal1500.template` passed navigation, native text input (`InkFast`) and search submission; Hiragana, Katakana, Kanji, combining/mixed text; coloured emoji including skin tones and sequences; content dragging; scrollbar-thumb dragging in both directions; confirmation; and error/retry state changes. The error page changed to “The example loaded successfully.” after retry. Screenshots and OCR are retained under `template/`; the runtime-error log was empty. This was a focused smoke check, not exhaustive coverage of every template component. The user's normal template package was not replaced.

## Rejected scheduling experiments

Diagnostic APKs varied one scheduling choice at a time; each had one warm-up and two measured probes. These were screening experiments, not full acceptance comparisons. `scheduling-experiments.json.gz` preserves every sample. No failed candidate was retained.

- Removing scoped performance-core affinity (`experiment-20260928-111724-99885146`) substantially regressed measured medians to 61.03/48.80 ms, with p95 66.50/64.96 ms. Reverted.
- Keeping the JavaScript affinity between messages (`experiment-20260928-111910-57aa212f`) did not remove the slow tail: medians 12.75/11.71 ms; p95 28.58/27.04 ms. The following original-build controls had medians 13.00/12.34 ms. Reverted.
- Giving the JavaScript thread nice -4 (`experiment-20260928-112121-58768ecf`) also did not remove the tail: medians 12.30/13.33 ms; p95 27.53/27.98 ms. `/proc` confirmed that nice -4 was applied. Reverted. This is the numeric priority Android documents for [display work](https://developer.android.com/reference/android/os/Process#THREAD_PRIORITY_DISPLAY).

- A temporary scoped `sched_setattr` utilisation-minimum request of 640/1024 (`experiment-20260928-112734-fd5b7bc1`) also failed to remove the slow tail: medians 11.67/12.51 ms; p95 28.28/27.96 ms. A separate trace still showed slow JavaScript execution around 695 MHz. This does not establish whether the kernel rejected or ignored the request; no successful application of the hint was verified. Reverted. Raw probes and trace analysis are retained in `util-hint.json.gz` and `util-hint-js-cpu.json`.

## Updated headless attribution

A fresh coarse diagnostic run (`headless-20260928-112958-d7b21438`) checks the current retained code after reverting scheduling prototypes. Component execution averaged 0.310 ms, the remaining React work loop 0.705 ms, commit 0.263 ms, effects 0.047 ms and native batch transfer 0.043 ms. These diagnostic timings include instrumentation; they are not a new acceptance comparison. Full per-update output is in `headless-profile.json.gz`. The unchanged fixture's labels, geometry, scrolling, reverse, hide/show and count-change checks passed.

A separate Android `-mtune=cortex-a78` build (`experiment-20260928-113442-2983e7f6`) had measured medians 12.83/12.03 ms and p95 28.10/26.38 ms. This short screen did not establish a benefit over the retained build, so the flag was removed. Its samples are in `tune-a78.json.gz`. All scheduling and compiler prototypes from this session are reverted.

## One-second CPU iteration on the LP3

The existing headless tool now accepts `--serial`, `--token` and `--ndk`. It cross-compiles the same harness and runs through ADB, without installing an Activity or tapping the screen. The Android driver uses the existing scoped performance-core preference, matching native input/update work; no new engine scheduling policy is involved. The runtime, affinity helper and compiler configuration hashes match the retained APK inputs.

The ordinary 100-update run `headless-20260928-114535-e22ca8e5` completed its entire checked suite in **1.032 seconds**, with **7.431 ms mean** and **7.724 ms p95** CPU input-to-scene duration. The companion diagnostic run `headless-20260928-114429-b9f75401` completed in 1.017 seconds and attributed about 2.842 ms to the React work-loop remainder, 1.407 ms to component execution, 1.279 ms to commit, 0.210 ms to effects and 0.267 ms to native transfer. Both passed every 500-cell scene, resize, reverse, hide/show and scrolling check. These were validation runs of the new tool, not a paired optimisation result.

These results run as Android shell with a continuous workload. They exclude input delivery, Activity scheduling, GPU submission and physical display latency. They do **not** meet or replace the full-app p95 acceptance gate. Use them for fast on-device CPU comparisons, then retain changes only after paired full-app checks. Raw results and reports are `android-headless.*` and `android-profile.*`.

The first cross-build attempt failed because bindgen had no Android sysroot; the tool now supplies the selected NDK's sysroot/include directory, as the normal APK build does. A preliminary diagnostic without the driver's scoped core preference also passed correctness but measured different scheduling (23.57 ms mean); it is not a comparable app CPU measurement. Its operation remains `headless-20260928-114159-57a6d693`.

The remote harness scratch directories were removed after each run. The device reservation was released, benchmark apps removed and saved device settings restored. Rejected experiment workspaces were cleaned; APKs, sources and measurements remain available. No production engine change was retained from this session.

## Mac tooling regression check

The new host mode completed its normal 200-update checks and seven Hyperfine runs. Its first timing result was 1.552 ms mean / 1.683 ms p95, with whole-process Hyperfine 372.3 ± 5.1 ms. A six-process-per-build paired comparison then reran both the saved overnight acceptance binary and the current binary. The means were 1.689 ms (old) and 1.682 ms (current), with p95 1.886/1.947 ms. Both exceed the narrow 1.5 ms mean target in this daytime series; the historical 1.498 ms result must not be presented as a repeatable guarantee under current host conditions. No timing samples were discarded. The current source hashes match the new run. The full goal remains incomplete. Raw paired samples and summary are retained beside this report.

## Interpreter follow-up

Native CPU sampling and two rejected property-access prototypes are documented in [the interpreter investigation](interpreter/README.md). Neither established a dependable LP3 gain; the upstream QuickJS dependency was restored. All paired fixture checks passed.

The first fusion comparison was invalid: Cargo reused the earlier cache binaries. The interpreter report records the matching hashes and superseding comparisons. Correctly rebuilt fusion improved phone headless mean by only 0.52%, with no p95 improvement, and was rejected. The benchmark tool now checks copied QuickJS C/header inputs against the resolved dependency. Restored Mac and Android binaries and bundles match their earlier production baselines byte for byte. All scene checks passed; current standalone Mac timing is 1.514 ms mean / 1.621 ms p95, and Android shell timing is 7.326 / 7.584 ms. These validation samples do not change the failed full-app p95 gate.

## Further screens

Physical-phone [compiler comparisons](compiler-screen/README.md) found no useful gain from disabling loop unrolling or omitting frame pointers. A [React persistence prototype](persistence-screen/README.md) passed the headless fixture but was 34.2% slower in paired Mac runs. Both experiments were rejected; the renderer and compiler configuration retain their prior behaviour.

## Native lookup and update-model follow-up

A [small retained Rust refactor](rust-targets/README.md) reduced native apply/layout mean by about 8% on Mac and 3% in the LP3 CPU harness. Its full-app p95 remains 27.147 ms, so the original goal is still incomplete. The candidate adds 80 APK bytes and matches 134 existing native scene outputs exactly.

A separate [direct-update diagnostic](direct-ui/README.md) keeps app state and calculations in JavaScript, but sends changed values directly to persistent native controls. Paired CPU means are 1.608 → 0.278 ms on Mac and 7.251 → 2.945 ms on LP3. This is a specialised non-React screen, not a production change or proof of React/template compatibility. It points towards a small binding API over the existing Rust engine. It does not establish full-app or display latency.

Two temporary [QuickJS allocation screens](allocation-screen/README.md) were not retained. The normal upstream QuickJS dependency and existing React adapter remain selected.
