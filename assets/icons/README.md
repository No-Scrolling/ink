# Material Symbols

Ink uses Material Symbols Outlined at grade 0 and optical size 24. Header and general icons use weight 400, fill 0. Bottom navigation uses weight 400, fill 1. Keyboard vectors use weight 300. Only referenced symbols enter the APK; the font and unused symbols are excluded.

The current snapshot is from Google `material-design-icons` commit `84ccef280841abfac506afc4ad4a2782f6d0a1d0`. Refresh the compiler font, codepoint map and keyboard vectors with:

```sh
uv run --with fonttools scripts/update_material_symbols.py
```

The source is licensed under Apache 2.0; see `assets/licenses/Material-Symbols-Apache-2.0.txt`.
