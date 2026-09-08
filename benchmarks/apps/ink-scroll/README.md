# Renderer scroll fixture

Opens directly into a virtualised list of 500 album rows with titles, subtitles and shared local artwork. No network, timers or navigation setup.

Use `scripts/agent-tools experiment` to build baseline and candidate revisions, then `bench --scenario scroll --rounds 3` to compare them. The app ID is separate from the older scroll benchmark so an existing installation is preserved.

The workload measures cached-image scrolling. Cold launches also exercise glyph and image uploads, but this fixture does not measure a stream of distinct thumbnails or image-cache eviction. Android launch timings are activity-start measurements, not proof of when artwork first becomes visible. Keep emulator results separate from LP3 results.
