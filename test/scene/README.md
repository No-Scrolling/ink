# Scene contracts

Run `bun run test scene`. Use `--profile release` to run the same contracts with release Rust binaries. Fixture preparation calls the real `ink-compiler`, including TypeScript checking, generated routes and native list transforms. The generated bundles run in QuickJS and feed actual commits into `ReactTree` and `Engine` at a 1080 × 1240 viewport.

`test/native/tests/scene.rs` exercises two compiled fixture applications:

- `lists`: public `List`, captured state and row callbacks, 2,000 rows, native scrolling without JavaScript window messages, bounded retained hosts, prepend anchors, reordering, emptying and genuine QuickJS Proxy fallback. Held-pointer cases cover updated callbacks and deletion without activating a replacement.
- `input`: public React `TextInput`, native editor updates, delayed React commit rejection, event counters, submit delivery and UTF-16 surrogate boundaries.

Expected row order, labels, event keys, indices and state are supplied independently of the production algorithms. Tap coordinates come from scene text rectangles intersected with the actual screen scroll clip, rather than overscan alone. Scene readiness waits use observed commits and ten-second deadlines.

These contracts validate scene data and interaction routing. They do not make pixel, typography, GPU or device-rendering claims. Any visual defect needs the repository's saved-screenshot pixel checks before being reported.

`test/native/tests/themes.rs` runs the compiled `themes` fixture, whose `theme.ts` uses the large header, Back at the end, `cutout: "below"`, inline fields, sections and a custom toggle. It checks title and Back placement against the theme's pixel values, layout under a simulated cutout, inline value wrapping, section grouping, theme colours against explicit canvas colours, and the disabled toggle image. Theme fonts are process-wide, so this runs as a separate test binary.

The compiled `navigation` fixture covers page/tab history, retained state and notification routing. See [navigation contracts](navigation-lifecycle.md).
