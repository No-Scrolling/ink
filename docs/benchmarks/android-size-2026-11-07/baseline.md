# Headless performance

Release build on Apple M4 Pro (darwin/arm64). 5 fresh processes, 20 warm-ups and 100 measured operations per workload/process.

| Workload | Median ms | p95 ms | p99 ms | Mean nodes measured |
| --- | ---: | ---: | ---: | ---: |
| react-small | 0.939 | 2.427 | 3.416 | 0.0 |
| react-bulk | 1.074 | 1.851 | 3.027 | 0.0 |
| react-resize | 2.089 | 2.903 | 5.031 | 1551.0 |
| list-edit-100 | 0.138 | 0.545 | 2.328 | 7.0 |
| list-scroll-100 | 0.015 | 0.042 | 0.048 | 5.9 |
| list-edit-1000 | 0.194 | 2.004 | 5.880 | 7.0 |
| list-scroll-1000 | 0.015 | 0.045 | 0.222 | 5.9 |
| list-edit-10000 | 0.730 | 4.078 | 10.913 | 7.0 |
| list-scroll-10000 | 0.015 | 0.019 | 0.020 | 5.9 |
| scrollbar-drag-10000 | 0.077 | 0.083 | 0.092 | 212.3 |
| rows-edit-100 | 0.167 | 1.268 | 4.884 | 27.0 |
| rows-edit-1000 | 0.243 | 1.734 | 4.244 | 27.0 |
| rows-prepend-1000 | 1.629 | 6.543 | 16.357 | 742.5 |
| rows-reorder-1000 | 1.573 | 4.999 | 7.977 | 366.0 |
| rows-native-scroll-1000 | 0.078 | 0.086 | 0.090 | 271.6 |
| rows-react-scroll-1000 | 0.828 | 2.649 | 3.829 | 347.8 |
| rows-pure-scroll-1000 | 0.114 | 0.128 | 0.146 | 347.8 |
| rows-decoded-image-arrival | 0.003 | 0.009 | 0.024 | 0.0 |
| startup-runtime-to-library | 3.822 | 7.928 | 9.658 | 3.0 |
| navigation-open | 1.173 | 3.731 | 7.128 | 702.0 |
| navigation-back | 0.301 | 1.171 | 5.959 | 3.0 |
| navigation-tab | 0.211 | 0.629 | 1.304 | 2.0 |
| conversation-controlled-input | 0.196 | 0.642 | 2.439 | 16.0 |
| conversation-append | 0.607 | 2.086 | 4.327 | 882.7 |
| conversation-prepend | 0.458 | 1.299 | 1.772 | 1067.0 |
| conversation-input-background | 0.605 | 2.534 | 4.729 | 1592.1 |
| conversation-input-under-load | 1.455 | 6.020 | 17.376 | 4422.0 |
| playback-native-clock | 0.000 | 0.000 | 0.000 | 0.0 |
| playback-state-change | 0.068 | 0.206 | 0.363 | 26.0 |
| conversation-input-cpu | 97.204 | 507.580 | 546.542 | NaN |

Scheduled-input phase timings distinguish late host dispatch from subsequent runtime/scene acknowledgement. Input dispatch includes native editing, event construction and enqueueing; it is not Android IME latency.

| Workload | Dispatch lateness p95 ms | Dispatch to acknowledgement p95 ms | Input dispatch p95 ms |
| --- | ---: | ---: | ---: |
| conversation-input-background | 0.361 | 1.850 | 0.268 |
| conversation-input-under-load | 0.528 | 5.728 | 0.174 |


Thumb update measures pointer movement through the native thumb geometry used by the renderer. Ready-scene timing also includes row window materialisation. GPU submission and display presentation are excluded.

| Workload | Thumb median ms | Thumb p95 ms | Thumb p99 ms | Ready-scene median ms |
| --- | ---: | ---: | ---: | ---: |
| scrollbar-drag-10000 | 0.007 | 0.007 | 0.009 | 0.077 |


All workload correctness/work contracts and idle observations passed. Updates cover dispatch through native scene application. Navigation includes native hit testing/back and viewport readiness. Controlled input covers native editing through the JS acknowledgement applied to the scene. Loaded input includes waiting from its prescribed arrival time. Scroll samples include JS window materialisation for custom React rows. Startup covers a fresh runtime/engine with source already read; image arrival starts with decoded RGBA; playback clocks bypass JS. These timings exclude Android process launch, IME, services, image decoding, GPU work and presentation. Native core instrumentation is enabled. Full correctness assertions run outside samples; scene readiness detection remains inside the measured path.

This is a dirty-tree measurement with source hashes, not a committed-source reproduction claim. The saved executable and compiled assets allow alternating baseline comparisons. Timing changes are reported without an automatic speed threshold.
