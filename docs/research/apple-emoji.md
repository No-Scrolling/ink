# Apple emoji in Ink

Research date: 30 August 2026.

## Project decision

Ink deliberately bundles an atlas containing only the 24 Light Keyboard emoji. The legal caveat below was reviewed before that decision; the implementation keeps the artwork behind a replaceable atlas seam and does not bundle the full Apple font.

## Original recommendation

Do not bundle Apple Color Emoji, or PNGs extracted from it, without explicit permission from Apple. Keep Light Keyboard's 24 emoji choices, but render them with the LP3's Android system emoji font. That is already how Light Keyboard obtains its emoji artwork, so Ink can match the existing keyboard on the target device without adding an emoji font or artwork to each APK.

For the lightweight keyboard, Android Canvas can draw the emoji labels directly. For emoji inserted into Ink's Rust-rendered text, add a narrow Android adapter that rasterises an emoji grapheme with the same system font into RGBA pixels; cache the result in the existing renderer as a colour texture. The keyboard press treatment can scale and translate the cached glyph and adds no asset dependency.

This is a technical and licensing-risk assessment, not legal advice.

## What `apple-emoji-ttf` provides

[`apple-emoji-ttf`](https://github.com/samuelngs/apple-emoji-ttf/tree/484daf4e13942437e083d881cae39fbc92d837e1) is a conversion pipeline. It reads Apple's `Apple Color Emoji.ttc`, extracts its `sbix` PNG strikes, and builds Linux/Windows CBDT/CBLC fonts or split web fonts with GSUB shaping ([pipeline](https://github.com/samuelngs/apple-emoji-ttf/blob/484daf4e13942437e083d881cae39fbc92d837e1/pipeline.py), [`sbix` extraction](https://github.com/samuelngs/apple-emoji-ttf/blob/484daf4e13942437e083d881cae39fbc92d837e1/source/sbix.py), [build recipes](https://github.com/samuelngs/apple-emoji-ttf/tree/484daf4e13942437e083d881cae39fbc92d837e1/configs)). It does not provide an independently licensed emoji design.

The current upstream release publishes complete converted fonts and Linux packages. Its release assets are large:

| Asset | Bytes | Approximate size |
|---|---:|---:|
| macOS source `Apple Color Emoji.ttc` (local macOS 26 installation) | 192,120,460 | 183.2 MiB |
| `AppleColorEmoji-Linux.ttf` | 115,969,256 | 110.6 MiB |
| `AppleColorEmoji-Windows.ttf` | 256,391,076 | 244.5 MiB |

The release sizes come from the project's [latest-release API](https://api.github.com/repos/samuelngs/apple-emoji-ttf/releases/latest). Its README also describes whole-font release packages and requires the macOS TTC as the build input ([README](https://github.com/samuelngs/apple-emoji-ttf/blob/484daf4e13942437e083d881cae39fbc92d837e1/README.md#build-the-font-yourself)). Bundling one of these files is incompatible with Ink's size goal.

## Licence and redistribution caveat

The repository's MIT licence applies only to its code. The project explicitly says that it is educational, Apple's assets and designs belong to Apple, and Apple Color Emoji assets are excluded from the MIT grant ([README disclaimer and licence](https://github.com/samuelngs/apple-emoji-ttf/blob/484daf4e13942437e083d881cae39fbc92d837e1/README.md#disclaimer)). The fact that converted fonts are downloadable from a GitHub release does not itself grant Ink redistribution rights.

Apple's macOS licence treats fonts as part of the Apple Software. It permits displaying and printing with the fonts while Apple Software is running, and permits embedding only where the font's own embedding restrictions allow it; the same licence generally prohibits redistribution and unauthorised copying or modification of Apple Software ([macOS Sequoia SLA, sections 1A, 2E, 2J and 2N](https://www.apple.com/legal/sla/docs/macOSSequoia.pdf)).

The locally installed Apple Color Emoji font reports `OS/2.fsType = 0x0004`. The OpenType specification defines that value as **Preview & Print embedding**, where an embedded font is temporarily loaded to view or print a read-only document—not installable redistribution in an Android application ([OpenType `OS/2.fsType`](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#fstype)). Notably, the conversion project's web recipe changes this field to `installable`, and its Windows recipe changes it to `editable` ([web recipe](https://github.com/samuelngs/apple-emoji-ttf/blob/484daf4e13942437e083d881cae39fbc92d837e1/configs/web.yaml), [Windows recipe](https://github.com/samuelngs/apple-emoji-ttf/blob/484daf4e13942437e083d881cae39fbc92d837e1/configs/windows.yaml)); changing font metadata does not create a licence from Apple.

Extracting only 24 PNG glyphs makes the payload smaller, but it still reproduces and distributes Apple's artwork. Subsetting therefore solves size, not rights.

## The actual Light Keyboard set

Light Keyboard does not contain an Apple font or emoji artwork. It defines these 24 labels and renders them as ordinary Compose text, allowing Android's font fallback to supply their appearance ([emoji list](/Users/vandam/Developer/light-keyboard/ui/src/main/java/com/thelightphone/lp3Keyboard/ui/viewmodel/Lp3KeyboardViewModel.kt), [emoji layout](/Users/vandam/Developer/light-keyboard/ui/src/main/java/com/thelightphone/lp3Keyboard/ui/layout/EnShared.kt)):

```text
😅 ☺️ 🙃 😍 😜 😂 😭 😎
🙌 👍 👎 🤞 ✌️ 👌 👋 🙏
✨ 🔥 ❤️ 💔 🏆 🎯 👑 👀
```

The current implementation converts every label with `codePointAt(0)`, so it drops the variation selector from `☺️`, `✌️` and `❤️` ([source](/Users/vandam/Developer/light-keyboard/ui/src/main/java/com/thelightphone/lp3Keyboard/ui/viewmodel/Lp3KeyboardViewModel.kt)). Ink should model emoji edits as strings/grapheme clusters, not integer code points. This retains emoji presentation and lets backspace remove one perceived character atomically.

## Curated Apple size experiment

Using commit `484daf4e`'s `source.sbix.get_emoji_png` against the local macOS 26 TTC, the raw PNG payload for Light Keyboard's 24 first code points measured:

| Apple bitmap strike | Combined PNG bytes |
|---|---:|
| 20 ppem | 23,727 (23.2 KiB) |
| 32 ppem | 47,464 (46.4 KiB) |
| 40 ppem | 65,657 (64.1 KiB) |
| 64 ppem | 131,446 (128.4 KiB) |
| 96 ppem | 237,743 (232.2 KiB) |

A FontTools subset retaining all of the source font's bitmap resolutions was 1,261,796 bytes, of which 1,259,793 bytes were the multi-resolution `sbix` table. A single appropriately sized strike or atlas is therefore the technically minimal representation—but these measurements do not alter the redistribution caveat.

## Feasible rendering paths

### 1. LP3 system emoji — recommended

- Preserve the 24 Unicode grapheme strings above.
- Draw keyboard labels with Android Canvas and the device's font fallback.
- Rasterise those same graphemes through a tiny Android-to-Rust adapter for Ink text, then cache/upload them to an RGBA atlas.
- Bundle no emoji font and no emoji artwork.

Android devices already include a system colour emoji font, and AOSP documents `NotoColorEmoji.ttf` in the system fallback chain ([AOSP font fallback](https://source.android.com/docs/core/fonts/custom-font-fallback)). Because both renderers use the same LP3 system font, this route matches Light Keyboard on the actual target rather than attempting to imitate Apple.

Ink's current renderer only uploads monochrome coverage from Public Sans outlines, so colour emoji requires an RGBA texture path regardless of which artwork is chosen ([renderer](/Users/vandam/Developer/ink/crates/ink-renderer-wgpu/src/compact.rs)). `ab_glyph` can expose pre-rendered colour glyph images, but Ink does not currently consume that API ([`Font::glyph_raster_image2`](https://docs.rs/ab_glyph/latest/ab_glyph/trait.Font.html#tymethod.glyph_raster_image2)). A curated runtime atlas is a smaller change than introducing a complete font fallback and shaping stack.

### 2. Curated Noto assets — deterministic, permissively licensed

If identical visuals across different Android versions matter more than matching the device, bundle only the 24 Noto SVG/PNG assets. Google's repository provides the source images under Apache 2.0 and the complete fonts under OFL 1.1 ([Noto Emoji repository](https://github.com/googlefonts/noto-emoji), [SVG licence](https://github.com/googlefonts/noto-emoji/blob/main/svg/LICENSE)). This is legally tractable and small, but it will look like Noto rather than Apple.

Do not bundle the full compatibility font: Android's own guidance says `NotoColorEmoji` exceeds 10 MB and recommends avoiding the bundled Emoji2 font where possible ([Emoji2 guidance](https://developer.android.com/develop/ui/views/text-and-emoji/emoji2#support-bundled-fonts)).

### 3. Curated Twemoji assets

Twemoji provides individual SVG and 72×72 PNG assets. Its graphics are CC BY 4.0, so attribution must accompany distribution ([Twemoji repository and licence](https://github.com/jdecked/twemoji)). This is also compact and deterministic, but visually further from Apple's set.

### 4. Apple artwork with permission

If the Apple appearance is non-negotiable, obtain explicit licensing or written permission from Apple, then ship a curated single-resolution atlas rather than a converted whole font. Until then, keep any Apple extraction experiment local and out of Ink releases and generated apps.

## Proposed Ink boundary

Keep one framework-level representation and two render adapters:

```text
Emoji grapheme string
       |
       +-- Android Canvas keyboard label
       |
       +-- Android system rasteriser -> cached RGBA texture -> Ink text
```

The app-facing API does not need to expose the source of the artwork. It only needs grapheme-correct insertion, deletion and submission. This keeps the legal/art choice replaceable without leaking it into every `TextInput` or keyboard layout.
