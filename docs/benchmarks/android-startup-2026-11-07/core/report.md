# Headless performance

Release build on Apple M4 Pro (darwin/arm64). 5 fresh processes, 20 warm-ups and 100 measured operations per workload/process.

| Workload | Median ms | p95 ms | p99 ms | Mean nodes measured |
| --- | ---: | ---: | ---: | ---: |
| react-small | 0.969 | 2.684 | 3.728 | 0.0 |
| react-bulk | 1.111 | 3.221 | 7.011 | 0.0 |
| react-resize | 2.343 | 5.545 | 8.345 | 1551.0 |
| list-edit-100 | 0.148 | 0.376 | 0.473 | 7.0 |
| list-scroll-100 | 0.015 | 0.019 | 0.067 | 5.9 |
| list-edit-1000 | 0.203 | 0.595 | 0.956 | 7.0 |
| list-scroll-1000 | 0.014 | 0.018 | 0.018 | 5.9 |
| list-edit-10000 | 0.769 | 2.507 | 3.635 | 7.0 |
| list-scroll-10000 | 0.015 | 0.025 | 0.079 | 5.9 |
| scrollbar-drag-10000 | 0.078 | 0.091 | 0.142 | 212.3 |
| rows-edit-100 | 0.185 | 0.413 | 0.635 | 27.0 |
| rows-edit-1000 | 0.283 | 0.781 | 1.162 | 27.0 |
| rows-prepend-1000 | 1.732 | 2.348 | 3.905 | 742.5 |
| rows-reorder-1000 | 1.645 | 2.473 | 4.255 | 366.0 |
| rows-native-scroll-1000 | 0.080 | 0.092 | 0.130 | 271.6 |
| rows-react-scroll-1000 | 0.847 | 1.408 | 2.505 | 347.8 |
| rows-pure-scroll-1000 | 0.114 | 0.137 | 0.197 | 347.8 |
| rows-decoded-image-arrival | 0.003 | 0.006 | 0.011 | 0.0 |
| startup-runtime-to-library | 4.015 | 5.121 | 7.469 | 3.0 |
| navigation-open | 1.297 | 3.085 | 4.593 | 702.0 |
| navigation-back | 0.362 | 1.491 | 6.268 | 3.0 |
| navigation-tab | 0.231 | 0.743 | 1.336 | 2.0 |
| conversation-controlled-input | 0.239 | 1.122 | 2.333 | 16.0 |
| conversation-append | 0.698 | 2.059 | 2.650 | 882.7 |
| conversation-prepend | 0.517 | 1.494 | 2.502 | 1067.0 |
| conversation-input-background | 0.682 | 2.866 | 8.219 | 1592.1 |
| conversation-input-under-load | 1.886 | 64.676 | 95.149 | 4420.8 |
| playback-native-clock | 0.000 | 0.000 | 0.000 | 0.0 |
| playback-state-change | 0.081 | 0.358 | 0.688 | 26.0 |
| conversation-input-cpu | 139.545 | 507.111 | 539.188 | NaN |

Scheduled-input phase timings distinguish late host dispatch from subsequent runtime/scene acknowledgement. Input dispatch includes native editing, event construction and enqueueing; it is not Android IME latency.

| Workload | Dispatch lateness p95 ms | Dispatch to acknowledgement p95 ms | Input dispatch p95 ms |
| --- | ---: | ---: | ---: |
| conversation-input-background | 0.577 | 2.579 | 0.273 |
| conversation-input-under-load | 0.821 | 64.596 | 0.213 |


Thumb update measures pointer movement through the native thumb geometry used by the renderer. Ready-scene timing also includes row window materialisation. GPU submission and display presentation are excluded.

| Workload | Thumb median ms | Thumb p95 ms | Thumb p99 ms | Ready-scene median ms |
| --- | ---: | ---: | ---: | ---: |
| scrollbar-drag-10000 | 0.007 | 0.008 | 0.032 | 0.078 |


Baseline and candidate were rerun in alternating order. Negative changes mean faster. Per-round changes and raw samples remain in result.json.

| Workload | Median change % | p95 change % |
| --- | ---: | ---: |
| react-small | -18.0 | -20.9 |
| react-bulk | -20.1 | -9.9 |
| react-resize | -8.1 | -24.0 |
| list-edit-100 | -24.4 | -47.6 |
| list-scroll-100 | -1.7 | 0.9 |
| list-edit-1000 | -15.8 | -65.5 |
| list-scroll-1000 | -7.0 | -57.7 |
| list-edit-10000 | -4.9 | -8.3 |
| list-scroll-10000 | -0.8 | 27.1 |
| scrollbar-drag-10000 | -1.6 | -3.5 |
| rows-edit-100 | 0.7 | -28.2 |
| rows-edit-1000 | 2.9 | -9.5 |
| rows-prepend-1000 | -3.6 | -56.3 |
| rows-reorder-1000 | -4.3 | -51.6 |
| rows-native-scroll-1000 | 0.6 | -4.5 |
| rows-react-scroll-1000 | -4.9 | -28.4 |
| rows-pure-scroll-1000 | -1.4 | -2.7 |
| rows-decoded-image-arrival | -18.6 | -35.8 |
| startup-runtime-to-library | 1.2 | -27.0 |
| navigation-open | 2.0 | 2.6 |
| navigation-back | 2.8 | 8.9 |
| navigation-tab | 4.2 | 18.0 |
| conversation-controlled-input | 16.5 | 80.6 |
| conversation-append | 14.1 | -39.9 |
| conversation-prepend | 6.4 | -9.1 |
| conversation-input-background | -21.1 | -58.7 |
| conversation-input-under-load | -21.1 | 85.0 |
| playback-native-clock | 0.0 | 0.0 |
| playback-state-change | 9.9 | 42.8 |
| conversation-input-cpu | 25.8 | 135.7 |

All workload correctness/work contracts and idle observations passed. Updates cover dispatch through native scene application. Navigation includes native hit testing/back and viewport readiness. Controlled input covers native editing through the JS acknowledgement applied to the scene. Loaded input includes waiting from its prescribed arrival time. Scroll samples include JS window materialisation for custom React rows. Startup covers a fresh runtime/engine with source already read; image arrival starts with decoded RGBA; playback clocks bypass JS. These timings exclude Android process launch, IME, services, image decoding, GPU work and presentation. Native core instrumentation is enabled. Full correctness assertions run outside samples; scene readiness detection remains inside the measured path.

This is a dirty-tree measurement with source hashes, not a committed-source reproduction claim. The saved executable and compiled assets allow alternating baseline comparisons. Timing changes are reported without an automatic speed threshold.
