"""Build the bundled single-cell font from upstream NotoSansSymbols2-Regular.ttf.

uv run --with fonttools==4.66.1 python scripts/terminal-symbols.py /path/to/upstream.ttf
Upstream: https://github.com/google/fonts/tree/main/ofl/notosanssymbols2
License: app/src/main/assets/licenses/NotoSansSymbols2-OFL.txt
"""
from pathlib import Path
import sys

from fontTools import subset
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont

font = TTFont(sys.argv[1], recalcTimestamp=False)
options = subset.Options()
options.name_IDs = ["*"]
options.name_legacy = True
options.name_languages = ["*"]
subsetter = subset.Subsetter(options=options)
subsetter.populate(text="·✢✳✶✻✽" + "".join(chr(c) for c in range(0x23F4, 0x23FB)))
subsetter.subset(font)
assert font["head"].unitsPerEm == 1000

# Every symbol advances 0.6em. Fit wide symbols without distorting circles,
# and center all animation frames in the same cell and around the same height.
glyph_set = font.getGlyphSet()
replacements = {}
for name in set(font.getBestCmap().values()):
    glyph = font["glyf"][name]
    scale = min(1, 560 / (glyph.xMax - glyph.xMin))
    transform = (scale, 0, 0, scale,
                 300 - scale * (glyph.xMin + glyph.xMax) / 2,
                 350 - scale * (glyph.yMin + glyph.yMax) / 2)
    pen = TTGlyphPen(glyph_set)
    glyph_set[name].draw(TransformPen(pen, transform))
    replacements[name] = pen.glyph()
for name, glyph in replacements.items():
    font["glyf"][name] = glyph
    glyph.recalcBounds(font["glyf"])
    font["hmtx"][name] = (600, glyph.xMin)
# The full symbol font reserves room for unrelated tall glyphs. These marks
# fit inside a normal text row; blinking must not change its ascent/descent.
font["hhea"].ascent, font["hhea"].descent, font["hhea"].lineGap = 800, -200, 0
font["OS/2"].sTypoAscender, font["OS/2"].sTypoDescender = 800, -200
font["OS/2"].sTypoLineGap = 0
font["OS/2"].usWinAscent, font["OS/2"].usWinDescent = 800, 200
font["post"].isFixedPitch = 1
font.save(Path(__file__).resolve().parents[1] / "app/src/main/res/font/noto_sans_symbols2.ttf")
