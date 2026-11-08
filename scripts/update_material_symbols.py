from array import array
from copy import deepcopy
from io import BytesIO
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.request import urlopen
import subprocess
import zlib

from fontTools.ttLib import TTFont
from fontTools import subset
from fontTools.ttLib.tables._g_l_y_f import Glyph, GlyphCoordinates
from fontTools.ttLib.tables.ttProgram import Program
from fontTools.varLib.instancer import instantiateVariableFont


COMMIT = "84ccef280841abfac506afc4ad4a2782f6d0a1d0"
BASE_URL = f"https://raw.githubusercontent.com/google/material-design-icons/{COMMIT}/variablefont"
FONT_NAME = "MaterialSymbolsOutlined%5BFILL%2CGRAD%2Copsz%2Cwght%5D"


DERIVED_SEEK_CODEPOINTS = {"replay_15": 0xE005, "forward_15": 0xE006}


def add_seek_fifteens(font: TTFont) -> None:
    cmap = font.getBestCmap()
    source_codepoints = {"replay_5": 0xE05B, "replay_10": 0xE059,
                         "forward_5": 0xE058, "forward_10": 0xE056}

    def contours(glyph):
        start = 0
        for end in glyph.endPtsOfContours:
            yield list(glyph.coordinates[start:end + 1]), list(glyph.flags[start:end + 1])
            start = end + 1

    def contour_at(glyph, expected_bounds):
        for points, flags in contours(glyph):
            bounds = (min(x for x, y in points), min(y for x, y in points),
                      max(x for x, y in points), max(y for x, y in points))
            if bounds == expected_bounds:
                return points, flags
        raise ValueError(f"Seek source contour changed: expected {expected_bounds}")

    for direction in ("replay", "forward"):
        ten_name = cmap[source_codepoints[f"{direction}_10"]]
        ten = font["glyf"][ten_name]
        ring = contour_at(ten, (140, 100, 820, 888))
        one = contour_at(ten, (311, 326, 416, 554))
        points, flags = contour_at(font["glyf"][cmap[source_codepoints[f"{direction}_5"]]], (386, 326, 554, 554))
        five = ([(x + 78, y) for x, y in points], flags)
        glyph = Glyph()
        glyph.numberOfContours = 3
        glyph.coordinates = GlyphCoordinates()
        glyph.flags = array("B")
        glyph.endPtsOfContours = []
        ordered = [ring, one, five] if direction == "replay" else [one, five, ring]
        for points, flags in ordered:
            glyph.coordinates.extend(points)
            glyph.flags.extend(flag & 1 for flag in flags)
            glyph.endPtsOfContours.append(len(glyph.coordinates) - 1)
        # Keep the source contour order and first-point overlap marker so the
        # rasteriser follows the same antialiasing path for the shared ring.
        glyph.flags[0] |= ten.flags[0] & 64
        glyph.program = Program()
        glyph.program.fromBytecode([])
        glyph.recalcBounds(font["glyf"])
        name = f"{direction}_15"
        order = list(font.getGlyphOrder())
        font["glyf"][name] = glyph
        font["hmtx"][name] = deepcopy(font["hmtx"][ten_name])
        for table in font["cmap"].tables:
            if table.isUnicode():
                table.cmap[DERIVED_SEEK_CODEPOINTS[name]] = name
        font.setGlyphOrder(order + [name])


def download(suffix: str) -> bytes:
    with urlopen(f"{BASE_URL}/{FONT_NAME}.{suffix}") as response:
        return response.read()


def main() -> None:
    output = Path(__file__).resolve().parents[1] / "assets/icons"
    output.mkdir(parents=True, exist_ok=True)

    codepoints = download("codepoints").decode()
    upstream = dict(line.split() for line in codepoints.splitlines())
    for name, codepoint in DERIVED_SEEK_CODEPOINTS.items():
        if name in upstream or codepoint in {int(value, 16) for value in upstream.values()}:
            raise ValueError(f"Derived seek icon clashes with Material Symbols: {name} U+{codepoint:04X}")

    with TemporaryDirectory() as temporary_directory:
        source = Path(temporary_directory) / "MaterialSymbolsOutlined.ttf"
        source.write_bytes(download("ttf"))
        variants = (
            (0, 400, "MaterialSymbolsOutlined-400.ttf.zlib"),
            (1, 400, "MaterialSymbolsOutlined-Fill1-400.ttf.zlib"),
            (1, 300, "MaterialSymbolsOutlined-Seek-Fill1-300.ttf.zlib"),
        )
        for fill, weight, filename in variants:
            font = TTFont(source)
            if any(codepoint in font.getBestCmap() for codepoint in DERIVED_SEEK_CODEPOINTS.values()):
                raise ValueError("Derived seek codepoints clash with the source font")
            instantiateVariableFont(
                font,
                {"FILL": fill, "GRAD": 0, "opsz": 24, "wght": weight},
                inplace=True,
            )
            if weight == 300:
                subsetter = subset.Subsetter()
                subsetter.populate(glyphs=[f"{direction}_{seconds}" for direction in ("replay", "forward") for seconds in (5, 10, 30)])
                subsetter.subset(font)
                # Round the instantiated outlines through TrueType serialisation,
                # matching the saved source subset used for the approved design.
                rounded = BytesIO()
                font.recalcTimestamp = False
                font.save(rounded, reorderTables=False)
                font = TTFont(BytesIO(rounded.getvalue()))
                add_seek_fifteens(font)
            static_font = (
                Path(temporary_directory)
                / f"MaterialSymbolsOutlined-Fill{fill}-{weight}.ttf"
            )
            font.flavor = None
            font.recalcTimestamp = False
            font.save(static_font, reorderTables=False)
            compressed_font = zlib.compress(static_font.read_bytes(), level=9)
            (output / filename).write_bytes(compressed_font)

    entries = codepoints.rstrip().splitlines() + [
        f"{name} {codepoint:04x}" for name, codepoint in DERIVED_SEEK_CODEPOINTS.items()
    ]
    (output / "MaterialSymbolsOutlined.codepoints").write_text("\n".join(sorted(entries)) + "\n")
    subprocess.run(["bun", str(output.parents[1] / "scripts/generate-icons.ts")], check=True)


if __name__ == "__main__":
    main()
