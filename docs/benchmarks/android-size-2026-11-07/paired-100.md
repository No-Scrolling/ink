# Headless performance

Release build on Apple M4 Pro (darwin/arm64). 5 fresh processes, 20 warm-ups and 100 measured operations per workload/process.

| Workload | Median ms | p95 ms | p99 ms | Mean nodes measured |
| --- | ---: | ---: | ---: | ---: |
| react-small | 0.927 | 1.047 | 1.121 | 0.0 |
| react-bulk | 1.048 | 1.180 | 1.307 | 0.0 |
| react-resize | 2.151 | 2.367 | 2.578 | 1551.0 |
| list-edit-100 | 0.139 | 0.229 | 0.321 | 7.0 |
| list-scroll-100 | 0.014 | 0.018 | 0.018 | 5.9 |
| list-edit-1000 | 0.193 | 0.428 | 0.532 | 7.0 |
| list-scroll-1000 | 0.014 | 0.018 | 0.023 | 5.9 |
| list-edit-10000 | 0.718 | 2.167 | 2.490 | 7.0 |
| list-scroll-10000 | 0.015 | 0.019 | 0.027 | 5.9 |
| scrollbar-drag-10000 | 0.076 | 0.085 | 0.103 | 212.3 |
| rows-edit-100 | 0.169 | 0.314 | 0.397 | 27.0 |
| rows-edit-1000 | 0.244 | 0.541 | 0.743 | 27.0 |
| rows-prepend-1000 | 1.616 | 2.051 | 2.241 | 742.5 |
| rows-reorder-1000 | 1.544 | 1.905 | 2.052 | 366.0 |
| rows-native-scroll-1000 | 0.078 | 0.087 | 0.120 | 271.6 |
| rows-react-scroll-1000 | 0.809 | 0.931 | 1.369 | 347.8 |
| rows-pure-scroll-1000 | 0.113 | 0.130 | 0.160 | 347.8 |
| rows-decoded-image-arrival | 0.003 | 0.006 | 0.009 | 0.0 |
| startup-runtime-to-library | 3.708 | 3.917 | 4.052 | 3.0 |
| navigation-open | 1.086 | 1.382 | 1.534 | 702.0 |
| navigation-back | 0.296 | 0.405 | 0.586 | 3.0 |
| navigation-tab | 0.217 | 0.307 | 0.379 | 2.0 |
| conversation-controlled-input | 0.202 | 0.289 | 0.390 | 16.0 |
| conversation-append | 0.613 | 0.916 | 1.100 | 882.7 |
| conversation-prepend | 0.464 | 0.696 | 0.839 | 1067.0 |
| conversation-input-background | 0.530 | 1.296 | 1.608 | 1592.1 |
| conversation-input-under-load | 1.504 | 1.963 | 2.441 | 4422.0 |
| playback-native-clock | 0.000 | 0.000 | 0.000 | 0.0 |
| playback-state-change | 0.065 | 0.108 | 0.135 | 26.0 |
| conversation-input-cpu | 78.875 | 161.727 | 169.619 | NaN |

Scheduled-input phase timings distinguish late host dispatch from subsequent runtime/scene acknowledgement. Input dispatch includes native editing, event construction and enqueueing; it is not Android IME latency.

| Workload | Dispatch lateness p95 ms | Dispatch to acknowledgement p95 ms | Input dispatch p95 ms |
| --- | ---: | ---: | ---: |
| conversation-input-background | 0.240 | 1.138 | 0.107 |
| conversation-input-under-load | 0.251 | 1.863 | 0.087 |


Thumb update measures pointer movement through the native thumb geometry used by the renderer. Ready-scene timing also includes row window materialisation. GPU submission and display presentation are excluded.

| Workload | Thumb median ms | Thumb p95 ms | Thumb p99 ms | Ready-scene median ms |
| --- | ---: | ---: | ---: | ---: |
| scrollbar-drag-10000 | 0.007 | 0.008 | 0.009 | 0.076 |


Baseline and candidate were rerun in alternating order. Negative changes mean faster. Per-round changes and raw samples remain in result.json.

| Workload | Median change % | p95 change % |
| --- | ---: | ---: |
| react-small | -1.2 | -9.4 |
| react-bulk | -2.7 | -16.3 |
| react-resize | -1.2 | -28.6 |
| list-edit-100 | -1.2 | -41.1 |
| list-scroll-100 | -1.7 | -2.3 |
| list-edit-1000 | -2.6 | -19.5 |
| list-scroll-1000 | -0.3 | -2.7 |
| list-edit-10000 | -3.6 | -8.5 |
| list-scroll-10000 | 0.8 | 0.9 |
| scrollbar-drag-10000 | -0.3 | -6.5 |
| rows-edit-100 | -2.4 | 10.9 |
| rows-edit-1000 | -2.3 | -15.9 |
| rows-prepend-1000 | -2.2 | -5.8 |
| rows-reorder-1000 | -5.0 | -10.2 |
| rows-native-scroll-1000 | 0.2 | 0.4 |
| rows-react-scroll-1000 | -2.2 | -8.8 |
| rows-pure-scroll-1000 | -0.1 | -8.0 |
| rows-decoded-image-arrival | -15.6 | -9.9 |
| startup-runtime-to-library | -2.0 | -2.4 |
| navigation-open | -5.1 | -8.2 |
| navigation-back | -2.2 | -12.4 |
| navigation-tab | 0.2 | -20.6 |
| conversation-controlled-input | 1.4 | 4.7 |
| conversation-append | -0.1 | -0.8 |
| conversation-prepend | -0.3 | 4.4 |
| conversation-input-background | -7.9 | -14.1 |
| conversation-input-under-load | -4.1 | -29.2 |
| playback-native-clock | -1.2 | -40.2 |
| playback-state-change | -2.2 | -22.0 |
| conversation-input-cpu | 11.8 | 5.3 |

All workload correctness/work contracts and idle observations passed. Updates cover dispatch through native scene application. Navigation includes native hit testing/back and viewport readiness. Controlled input covers native editing through the JS acknowledgement applied to the scene. Loaded input includes waiting from its prescribed arrival time. Scroll samples include JS window materialisation for custom React rows. Startup covers a fresh runtime/engine with source already read; image arrival starts with decoded RGBA; playback clocks bypass JS. These timings exclude Android process launch, IME, services, image decoding, GPU work and presentation. Native core instrumentation is enabled. Full correctness assertions run outside samples; scene readiness detection remains inside the measured path.

This is a dirty-tree measurement with source hashes, not a committed-source reproduction claim. The saved executable and compiled assets allow alternating baseline comparisons. Timing changes are reported without an automatic speed threshold.
