# Rebuild with: uv run --with fonttools python generate.py
# Original test geometry, dedicated to the public domain (CC0).
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTCollection

fonts = []
for name, character in [('Latin', 'A'), ('Thai', 'ฅ')]:
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder(['.notdef', 'triangle'])
    builder.setupCharacterMap({ord(character): 'triangle'})
    pen = TTGlyphPen(None)
    pen.moveTo((50, 0))
    pen.lineTo((550, 0))
    pen.lineTo((300, 700))
    pen.closePath()
    builder.setupGlyf({'.notdef': TTGlyphPen(None).glyph(), 'triangle': pen.glyph()})
    builder.setupHorizontalMetrics({'.notdef': (600, 0), 'triangle': (600, 50)})
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable({'familyName': name, 'styleName': 'Regular',
                           'uniqueFontIdentifier': name, 'fullName': name,
                           'psName': name})
    builder.setupOS2(sTypoAscender=800, sTypoDescender=-200,
                    usWinAscent=800, usWinDescent=200)
    builder.setupPost()
    builder.setupMaxp()
    builder.font['head'].created = builder.font['head'].modified = 2082844800
    builder.font.recalcTimestamp = False
    fonts.append(builder.font)
collection = TTCollection()
collection.fonts = fonts
collection.save(Path(__file__).with_name('fallback.ttc'))
