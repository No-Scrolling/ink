# Rows, actions, pagination and barcode verification

6 September 2026. Template launched with `ink dev --device emulator-5554 --once`, using an agent-tools device reservation. No automated tests were added.

- Framework and template TypeScript checks, `cargo check -p ink-core` and `git diff --check` passed.
- Examples' more-horizontal icon opened Action Page with Action 1, 2 and 3. Action 1 and Back navigation were exercised.
- Rows displayed artwork, title/subtitle, wrapping preview text and a title-only row. Tapping the artwork opened Harbour after correcting the initial route typo. Evidence: `image-20260906-134628-879d2c96`, `image-20260906-134719-ebef73ff`.
- Pagination started with Item 0–19 loaded, then scrolling reached Item 43–47 and ultimately Item 95–99. The fixture stops at 100 items. No load-more button or loading indicator was shown. Evidence: `image-20260906-134722-0deda759`, `image-20260906-134737-b7486e65`, `image-20260906-134819-5a8934a0`.
- The centred Barcode screen rendered its QR image. Apple's Vision barcode detector decoded the emulator screenshot as exactly `Hello World!`. Evidence: `image-20260906-134823-e9be57b3`.
- Final process-filtered Ink/Android error log was empty. The initial bad row destination was corrected and retested.

Image evidence is local under `.agent-tools/`. Pagination uses delayed local pages; live network paging, failure/retry interaction, query replacement during an outstanding request and physical-phone QR scanning were not exercised in this pass. The pagination implementation serialises requests, deduplicates the current item boundary and exposes an error/retry on rejection; use a new List key when changing data sources.
