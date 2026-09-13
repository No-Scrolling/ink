from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.request import urlopen
import subprocess
import zlib

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont


COMMIT = "84ccef280841abfac506afc4ad4a2782f6d0a1d0"
BASE_URL = f"https://raw.githubusercontent.com/google/material-design-icons/{COMMIT}/variablefont"
FONT_NAME = "MaterialSymbolsOutlined%5BFILL%2CGRAD%2Copsz%2Cwght%5D"


def download(suffix: str) -> bytes:
    with urlopen(f"{BASE_URL}/{FONT_NAME}.{suffix}") as response:
        return response.read()


def main() -> None:
    output = Path(__file__).resolve().parents[1] / "assets/icons"
    output.mkdir(parents=True, exist_ok=True)

    with TemporaryDirectory() as temporary_directory:
        source = Path(temporary_directory) / "MaterialSymbolsOutlined.ttf"
        source.write_bytes(download("ttf"))
        variants = (
            (0, 400, "MaterialSymbolsOutlined-400.ttf.zlib"),
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
    subprocess.run(["bun", str(output.parents[1] / "scripts/generate-icons.ts")], check=True)


if __name__ == "__main__":
    main()
