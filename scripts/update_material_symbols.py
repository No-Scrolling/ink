from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.request import urlopen
import zlib

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont


COMMIT = "84ccef280841abfac506afc4ad4a2782f6d0a1d0"
BASE_URL = f"https://raw.githubusercontent.com/google/material-design-icons/{COMMIT}/variablefont"
FONT_NAME = "MaterialSymbolsOutlined%5BFILL%2CGRAD%2Copsz%2Cwght%5D"
KEYBOARD_SYMBOLS = {
    "ink_keyboard_arrow_down": "keyboard_arrow_down",
    "ink_keyboard_arrow_up": "keyboard_arrow_up",
    "ink_keyboard_chevron_left": "chevron_left",
    "ink_keyboard_done": "done",
    "ink_keyboard_match_case": "match_case",
    "ink_keyboard_mood": "mood",
    "ink_keyboard_return": "keyboard_return",
    "ink_keyboard_search": "search",
}


def download(suffix: str) -> bytes:
    with urlopen(f"{BASE_URL}/{FONT_NAME}.{suffix}") as response:
        return response.read()


def android_vector(name: str) -> str:
    url = (
        f"https://raw.githubusercontent.com/google/material-design-icons/{COMMIT}"
        f"/symbols/android/{name}/materialsymbolsoutlined/{name}_wght300_24px.xml"
    )
    with urlopen(url) as response:
        vector = response.read().decode()
    return vector.replace(
        '\n    android:tint="?attr/colorControlNormal">',
        ">",
    ).replace(
        '\n    android:tint="?attr/colorControlNormal"',
        "",
    ).replace(
        'android:fillColor="@android:color/white"',
        'android:fillColor="#FFFFFFFF"',
    )


def main() -> None:
    output = Path(__file__).resolve().parents[1] / "assets/icons"
    output.mkdir(parents=True, exist_ok=True)

    with TemporaryDirectory() as temporary_directory:
        source = Path(temporary_directory) / "MaterialSymbolsOutlined.ttf"
        source.write_bytes(download("ttf"))
        variants = (
            (0, 300, "MaterialSymbolsOutlined-300.ttf.zlib"),
            (1, 400, "MaterialSymbolsOutlined-Fill1-400.ttf.zlib"),
        )
        for fill, weight, filename in variants:
            font = TTFont(source)
            instantiateVariableFont(
                font,
                {"FILL": fill, "GRAD": 0, "opsz": 24, "wght": weight},
                inplace=True,
            )
            static_font = (
                Path(temporary_directory)
                / f"MaterialSymbolsOutlined-Fill{fill}-{weight}.ttf"
            )
            font.flavor = None
            font.recalcTimestamp = False
            font.save(static_font, reorderTables=False)
            compressed_font = zlib.compress(static_font.read_bytes(), level=9)
            (output / filename).write_bytes(compressed_font)

    (output / "MaterialSymbolsOutlined.codepoints").write_bytes(download("codepoints"))

    keyboard_output = (
        Path(__file__).resolve().parents[1]
        / "platform/android/app/src/textInput/res/drawable"
    )
    for output_name, symbol_name in KEYBOARD_SYMBOLS.items():
        (keyboard_output / f"{output_name}.xml").write_text(android_vector(symbol_name))


if __name__ == "__main__":
    main()
