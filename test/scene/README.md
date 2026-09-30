# Scene contracts

Run `bun run test scene`. Use `--profile release` to run the same contracts with release Rust binaries. Fixture preparation calls the real `ink-compiler`, including TypeScript checking, generated routes and native list transforms. The generated bundles run in QuickJS and feed actual commits into `ReactTree` and `Engine` at a 1080 × 1240 viewport.

`test/native/tests/scene.rs` exercises three compiled fixture applications:

- `lists`: public `List`, captured state and row callbacks, 2,000 rows, native scrolling without JavaScript window messages, bounded retained hosts, prepend anchors, reordering, emptying and genuine QuickJS Proxy fallback. Held-pointer cases cover updated callbacks and deletion without activating a replacement.
- `views`: public `nativeView`, native expressions, batched keyed collection insertion/update/move/removal, reset followed by an update, and native button and row events returned to JavaScript.
- `input`: public React `TextInput`, native editor updates, delayed React commit rejection, event counters, submit delivery and UTF-16 surrogate boundaries.

Expected row order, labels, event keys, indices and state are supplied independently of the production algorithms. Tap coordinates come from scene text rectangles intersected with the actual screen scroll clip, rather than overscan alone. Scene readiness waits use observed commits and ten-second deadlines.

These contracts validate scene data and interaction routing. They do not make pixel, typography, GPU or device-rendering claims. Any visual defect needs the repository's saved-screenshot pixel checks before being reported.

The compiled `navigation` and `lifecycle` fixtures cover page/tab history, retained state, notification routing and native-view subscription/action disposal. See [navigation and lifetime contracts](navigation-lifecycle.md).
