# Covers stalled after momentum scrolling

Confirmed on 28 September 2026 using Reverb in the Light Phone III emulator with 48 temporary album covers plus its existing album. The normal Reverb app and LP3 data were not changed.

## Reproduction

Start a fresh process, allow the album list to appear, then fling from `(520,1080)` to `(520,200)` over 60 ms. Leave the screen untouched at the bottom.

The baseline showed the final album titles but a completely black cover column for more than ten seconds. A slow 35-pixel upward scroll caused the covers to load. This matches the user's LP3 report.

- [Before the nudge](baseline-bottom.png): the crop `(45,145,140,915)` contains zero bright or coloured pixels. Evidence: `image-20260928-173014-fde8ab00`. OCR independently identifies the bottom album titles: `image-20260928-173014-7d01d98c`.
- [After the nudge](baseline-after-nudge.png): 93,698 pixels changed in that cover column; evidence `image-20260928-173025-c84dc28f`. This comparison includes the small scroll displacement.
- [Fixed, without a nudge](candidate-bottom.png): 74,365 coloured pixels in the same cover column, with covers recognised by OCR. Evidence: `image-20260928-173208-36a98662`; full before/after comparison `image-20260928-173208-2a74b5d9`.

## Cause and fix

`nativeScrollBy` can materialise native list rows and queue image requests during a fling. MainActivity previously drained those requests on touch processing, JavaScript commits or other resource completions. The momentum frame path did not drain them. Once outstanding requests finished, newly queued requests could remain idle indefinitely until another interaction.

Native-owned lists exposed this because updating their window no longer requires a React commit, which previously supplied an incidental request-drain opportunity.

`InkSurfaceView.presentFrame()` now calls `drainNativeRequests()` before rendering. This also covers queued work from captured thumb dragging. Existing asynchronous image completion already requests another frame when decoding finishes; that path is unchanged.

The additional local-artwork look-ahead from the preceding experiment was removed, including its 2 MiB speculative decode budget and synthetic prefetch check. Ordinary viewport overscan and existing image caching remain. The incremental list-data optimisation is retained.

## Validation

Baseline APK: `experiment-20260928-171849-4367d4e7`. Fixed APK: `experiment-20260928-173111-7cea584e`. Both use the same isolated Reverb package and album library. The fixed reproduction loads covers without further input; two additional fresh-process repeats capture the same condition after three seconds untouched.

Core checks (4), Ink TypeScript and the public-list headless regression pass (`headless-20260928-173139-56434f1e`), including zero list records transferred for unrelated state, one record per edit, fresh callbacks and React compatibility. The headless runner does not execute Android frame callbacks; the device reproduction is the relevant validation for this fix.

Phone verification is left to the user. Restart `ink dev --device LP3LHMA531900140` in Reverb to rebuild the Android implementation; the earlier isolated LP3 test APK does not contain this fix.
