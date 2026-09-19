# Renderer scroll fixture

Opens directly into 500 virtualised rows with varying heights: wrapping and truncated titles, optional subtitles and artwork, and Material and custom SVG subtitle icons. Rows have an eight-unit gap. No network, timers or navigation setup.

Use `scripts/agent-tools experiment` to build baseline and candidate revisions, then `bench --scenario scroll --rounds 3` to compare them. The app ID is separate from the older scroll benchmark so an existing installation is preserved.

Each illustrated row has unique, procedurally generated 256 × 256 artwork. Regenerate the images and their imports with `uv run benchmarks/apps/ink-scroll/generate-artwork.py`. The seed is the row number, so every run uses the same images.

Scrolling exercises image decoding and uploads as well as list layout. Android launch timings are activity-start measurements, not when artwork first becomes visible. Keep emulator results separate from LP3 results, and do not compare these mixed rows directly with older uniform-row results.
