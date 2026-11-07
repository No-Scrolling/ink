# Headless performance

Release build on Apple M4 Pro (darwin/arm64). 7 fresh processes, 40 warm-ups and 200 measured operations per workload/process.

| Workload | Median ms | p95 ms | p99 ms | Mean nodes measured |
| --- | ---: | ---: | ---: | ---: |
| react-small | 0.916 | 1.359 | 2.690 | 0.0 |
| react-bulk | 1.051 | 1.588 | 4.877 | 0.0 |
| react-resize | 2.070 | 2.489 | 3.572 | 1551.0 |
| list-edit-100 | 0.136 | 0.283 | 0.389 | 7.0 |
| list-scroll-100 | 0.014 | 0.018 | 0.020 | 5.6 |
| list-edit-1000 | 0.190 | 0.394 | 0.490 | 7.0 |
| list-scroll-1000 | 0.014 | 0.018 | 0.021 | 5.6 |
| list-edit-10000 | 0.700 | 1.998 | 2.425 | 7.0 |
| list-scroll-10000 | 0.015 | 0.018 | 0.023 | 5.6 |
| scrollbar-drag-10000 | 0.076 | 0.084 | 0.093 | 212.8 |
| rows-edit-100 | 0.171 | 0.267 | 0.385 | 27.0 |
| rows-edit-1000 | 0.243 | 0.499 | 0.620 | 27.0 |
| rows-prepend-1000 | 1.568 | 1.977 | 2.614 | 742.5 |
| rows-reorder-1000 | 1.537 | 1.992 | 2.443 | 366.0 |
| rows-native-scroll-1000 | 0.076 | 0.088 | 0.113 | 269.4 |
| rows-react-scroll-1000 | 0.813 | 1.049 | 1.365 | 344.1 |
| rows-pure-scroll-1000 | 0.111 | 0.133 | 0.210 | 344.1 |
| rows-decoded-image-arrival | 0.002 | 0.006 | 0.011 | 0.0 |
| startup-runtime-to-library | 3.693 | 4.168 | 4.458 | 3.0 |
| navigation-open | 1.099 | 1.458 | 1.612 | 702.0 |
| navigation-back | 0.287 | 0.355 | 0.481 | 3.0 |
| navigation-tab | 0.208 | 0.238 | 0.292 | 2.0 |
| conversation-controlled-input | 0.199 | 0.235 | 0.282 | 16.0 |
| conversation-append | 0.659 | 0.853 | 1.021 | 882.1 |
| conversation-prepend | 0.453 | 0.663 | 1.052 | 1067.0 |
| conversation-input-background | 0.574 | 1.862 | 2.843 | 1590.3 |
| conversation-input-under-load | 1.542 | 2.158 | 2.816 | 3930.6 |
| playback-native-clock | 0.000 | 0.000 | 0.000 | 0.0 |
| playback-state-change | 0.071 | 0.116 | 0.180 | 26.0 |
| conversation-input-cpu | 0.336 | 114.575 | 129.994 | NaN |

Scheduled-input phase timings distinguish late host dispatch from subsequent runtime/scene acknowledgement. Input dispatch includes native editing, event construction and enqueueing; it is not Android IME latency.

| Workload | Dispatch lateness p95 ms | Dispatch to acknowledgement p95 ms | Input dispatch p95 ms |
| --- | ---: | ---: | ---: |
| conversation-input-background | 0.247 | 1.744 | 0.117 |
| conversation-input-under-load | 0.254 | 1.989 | 0.086 |


Thumb update measures pointer movement through the native thumb geometry used by the renderer. Ready-scene timing also includes row window materialisation. GPU submission and display presentation are excluded.

| Workload | Thumb median ms | Thumb p95 ms | Thumb p99 ms | Ready-scene median ms |
| --- | ---: | ---: | ---: | ---: |
| scrollbar-drag-10000 | 0.007 | 0.008 | 0.009 | 0.076 |


Baseline and candidate were rerun in alternating order. Negative changes mean faster. Per-round changes and raw samples remain in result.json.

| Workload | Median change % | p95 change % |
| --- | ---: | ---: |
| react-small | -0.8 | 34.4 |
| react-bulk | -1.5 | 31.3 |
| react-resize | -1.3 | 8.7 |
| list-edit-100 | 0.5 | 41.3 |
| list-scroll-100 | -1.2 | -1.9 |
| list-edit-1000 | -0.8 | -5.9 |
| list-scroll-1000 | -1.5 | -3.4 |
| list-edit-10000 | -1.2 | -5.0 |
| list-scroll-10000 | -0.3 | -1.3 |
| scrollbar-drag-10000 | -0.5 | -6.5 |
| rows-edit-100 | 0.4 | -1.4 |
| rows-edit-1000 | 0.2 | -4.7 |
| rows-prepend-1000 | 0.6 | 1.4 |
| rows-reorder-1000 | 2.6 | 6.2 |
| rows-native-scroll-1000 | -0.3 | -3.3 |
| rows-react-scroll-1000 | 0.4 | -1.5 |
| rows-pure-scroll-1000 | -0.4 | -0.4 |
| rows-decoded-image-arrival | 13.7 | 4.1 |
| startup-runtime-to-library | 0.2 | 3.5 |
| navigation-open | 0.4 | 3.9 |
| navigation-back | -0.3 | -10.3 |
| navigation-tab | -0.9 | -12.6 |
| conversation-controlled-input | -1.0 | -8.6 |
| conversation-append | -3.9 | -14.5 |
| conversation-prepend | -0.9 | 2.3 |
| conversation-input-background | 0.0 | -2.5 |
| conversation-input-under-load | -4.2 | -6.8 |
| playback-native-clock | -32.8 | -39.9 |
| playback-state-change | -8.0 | 8.8 |
| conversation-input-cpu | -5.1 | 5.5 |

All workload correctness/work contracts and idle observations passed. Updates cover dispatch through native scene application. Navigation includes native hit testing/back and viewport readiness. Controlled input covers native editing through the JS acknowledgement applied to the scene. Loaded input includes waiting from its prescribed arrival time. Scroll samples include JS window materialisation for custom React rows. Startup covers a fresh runtime/engine with source already read; image arrival starts with decoded RGBA; playback clocks bypass JS. These timings exclude Android process launch, IME, services, image decoding, GPU work and presentation. Native core instrumentation is enabled. Full correctness assertions run outside samples; scene readiness detection remains inside the measured path.

This is a dirty-tree measurement with source hashes, not a committed-source reproduction claim. The saved executable and compiled assets allow alternating baseline comparisons. Timing changes are reported without an automatic speed threshold.
