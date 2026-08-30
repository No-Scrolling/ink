# Material Symbols

Ink uses the official Material Symbols Outlined set at grade 0 and optical size 24. General interface icons use fill 0 at weight 300, while bottom navigation icons use fill 1 at weight 400. The compiler rasterises only the symbols referenced by an application; neither font snapshot nor unused symbols enter its APK.

The current snapshot is from Google `material-design-icons` commit `84ccef280841abfac506afc4ad4a2782f6d0a1d0`. Refresh the compiler font, codepoint map and keyboard vectors with:

```sh
uv run --with fonttools scripts/update_material_symbols.py
```

The source is licensed under Apache 2.0; see `assets/licenses/Material-Symbols-Apache-2.0.txt`.
