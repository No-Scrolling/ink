# Wallet pass previews

`@ink/wallet` previews single `.pkpass` attachments on Android. `wallet.preview(source, signal)` accepts an HTTP(S) URL or an `ink-file://` managed file. Native code caches static previews by source, reusing the saved metadata and artwork before downloading or reading the ZIP. Missing artwork is rebuilt from the cached archive. Source keys are hashed, with up to eight source aliases per archive and 32 cached archives. `PassPreview` displays its fields and supported barcode using Ink components.

```tsx
const pass = await wallet.preview(attachment.src, signal);
<PassPreview pass={pass} />
if (await wallet.canOpen("com.vandamd.passes")) {
  await wallet.open(pass, "com.vandamd.passes");
}
```

Receiving apps implement an exported `com.vandam.ink.PassImportActivity`. The sender must declare the receiver package in Android `<queries>` for `canOpen`. The hand-off is an explicit `ACTION_VIEW` containing a FileProvider content URI, PKPass MIME type and temporary read permission. Copy the file while that permission is active. Passes implements this contract and asks the user before saving.

Conversation rows use `filePreview: { title, label, artwork? }` and `onFilePress(message)` to reuse the native link-card layout without representing an attachment as a web link. Artwork appears in a 56 × 56 square beside the labels, preserving its aspect ratio. Beeper loads pass artwork in the background and leaves the text card usable if no artwork is available.

Artwork: logo, strip, thumbnail, background, footer and icon PNGs, preferring @3x then @2x. Artwork is displayed as images alongside the fields, with background used when no strip is present. `wallet.retain(pass)` makes artwork durable before saving its returned metadata; `wallet.release(pass)` removes saved artwork when deleting a pass. Preview eviction does not remove retained artwork.

Layout: pass field groups drive paired primary fields (with a transit arrow for boarding passes), paired details and a centred barcode with its `altText` caption. The decorative logo/title row is hidden to keep the ticket compact. Header fields remain visible as ordinary detail rows; an unlabelled boarding-pass header that repeats the displayed route is omitted. Back fields are available through Passes’ action menu’s Info page; Beeper hides them. `wallet.details(pass)` returns labelled text parts with link destinations, parsing HTML anchors and recognising plain web and email links. Render linked parts with `Text href` for underlined, tappable links. Older saved passes without group metadata retain all fields in paired rows; reimport to restore the original grouping.

Supported: classic boarding pass, coupon, event ticket, generic and store card fields; QR, PDF417, Aztec and Code128 barcodes. UTF-8 is supported, along with ASCII data in recognised legacy encodings. Unsupported barcode formats or non-ASCII legacy encodings are omitted rather than silently changing the encoded data. Passes can still save the fields.

This is a static preview, not a full Apple Wallet implementation: no signature verification, exact Wallet layout reproduction, localisation, live updates, NFC or multi-pass archives. Date and time styles use the device locale, respecting `ignoresTimeZone`; invalid dates retain their source text. Numbers retain their source values. Cached previews are regenerated when the parser version changes. See [Apple's pass format](https://developer.apple.com/library/archive/documentation/UserExperience/Conceptual/PassKit_PG/Creating.html).

Limits: 10 MB per archive, 256 ZIP entries and 256 KB of `pass.json`, and 1 KB of barcode payload. Only the six recognised artwork filenames are read, with generated managed-file names. Each image is bounded to 2 MB, 4096 pixels per side and 8 megapixels. Only the 32 most recent archives remain in the preview cache; a missing cached archive requires reopening its preview. Passes stores the decoded fields/barcode separately, so a saved pass remains available offline.
