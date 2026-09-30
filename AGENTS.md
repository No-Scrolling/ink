## Rules

- Tests must protect an observable contract, catch a realistic defect and use independent expected results. Exercise the relevant production path; do not duplicate the implementation or mock the behaviour being tested.
- Use `emulator -avd Light_Phone_III -writable-system` for the emulator
- Before reporting a visual defect or changing rendering code, verify the saved screenshot by comparing pixels in the affected region. An image preview alone is not sufficient evidence. If the preview and file evidence disagree, investigate the inspection tooling first.

## Design exports and wireframes

- Use [ink export](docs/design-export.md) for actual routes or Ink compositions with repeatable fixtures. Freeze changing state and keep frames at 1080 × 1240 without scaling, added padding or borders.

If the user uses the tldraw plugin:

- Work on the user's open board and current page unless requested otherwise. Reuse frame names when updating images and preserve surrounding drawings.
- Keep flows to named frames and bound arrows unless additional text is requested. Use each arrow's own label and keep paths and labels clear of frames and other labels.
- Batch imports and layout changes, then fit the flow in the viewport. Verify the page and contents in a fresh board view and take a screenshot before reporting success.
- Return concise summaries without image data or full board snapshots.
