# Ink's ab_glyph patch

Source: ab_glyph 0.2.32 from crates.io, licensed under Apache-2.0.

Only the manifest's `std` feature differs from upstream: it enables
`owned_ttf_parser/std` instead of `owned_ttf_parser/default`. This keeps standard
floating-point operations without implicitly enabling variable fonts, layout
tables and glyph names. Ink uses fixed font instances and performs its own text
layout; TrueType and CFF outlines remain supported, including Android's static
Noto Sans Symbols and Noto Sans CJK fallback fonts.

The explicit `variable-fonts` and `gvar-alloc` features remain available. Remove
this patch when an upstream release allows `std` without the parser defaults.
