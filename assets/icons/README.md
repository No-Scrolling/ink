# Material Symbols

Ink uses Material Symbols Outlined at grade 0 and optical size 24. Header and general icons use weight 400, fill 0. Bottom navigation uses weight 400, fill 1. Filled 5-, 10-, 15- and 30-second seek icons use weight 300. Keyboard vectors use weight 300. Only referenced symbols enter the APK; the font and unused symbols are excluded.

The current snapshot is from Google `material-design-icons` commit `84ccef280841abfac506afc4ad4a2782f6d0a1d0`. Refresh the compiler font, codepoint map and keyboard vectors with:

```sh
uv run --with fonttools scripts/update_material_symbols.py
```

The 15-second seek icons are modified Material Symbols. For each direction, the refresh script copies the ring, arrow and `1` contours from the 10-second icon, then adds the `5` contour from that direction’s 5-second icon, shifted right by 78 font units without scaling. It preserves the source contour order, first-point overlap flag and metrics. The script checks the source contour bounds and rejects codepoint collisions before writing the fonts.

Ink assigns `replay_15` to U+E005 and `forward_15` to U+E006 in the private-use area. These codepoints are unused in the pinned Material snapshot. Only filled variants are exported.

The Material Symbols source and Ink’s derived glyphs are licensed under Apache 2.0; see `assets/licenses/Material-Symbols-Apache-2.0.txt`.
