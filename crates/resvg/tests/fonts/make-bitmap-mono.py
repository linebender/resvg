"""Build a small monochrome-bitmap test font from Terminus (TTF).

pyftsubset drops EBDT/EBLC, so the strike is pruned by hand and re-attached to
the subsetted font. The result is renamed because "Terminus" is an OFL Reserved
Font Name and this is a Modified Version.

Run next to TerminusTTF-Regular.ttf.
"""

from fontTools.subset import Options, Subsetter
from fontTools.ttLib import TTFont

SRC = "TerminusTTF-Regular.ttf"
DST = "BitmapMono.subset.ttf"
PPEM = 16
FAMILY = "Bitmap Mono"
PS_NAME = "BitmapMono-Regular"
# Space, digits and the latin alphabet.
CHARS = [0x20, *range(0x30, 0x3A), *range(0x41, 0x5B), *range(0x61, 0x7B)]

font = TTFont(SRC)
cmap = font.getBestCmap()
wanted = {cmap[c] for c in CHARS if c in cmap} | {".notdef"}

# The bitmap tables have to be taken from the original font, since subsetting
# drops them.
eblc, ebdt = font["EBLC"], font["EBDT"]
strike_index = next(
    i for i, s in enumerate(eblc.strikes) if s.bitmapSizeTable.ppemX == PPEM
)
strike = eblc.strikes[strike_index]
bitmaps = ebdt.strikeData[strike_index]

options = Options()
options.drop_tables += ["BDF ", "FFTM"]
subsetter = Subsetter(options=options)
subsetter.populate(glyphs=sorted(wanted))
subsetter.subset(font)

# Subsetting can pull in additional glyphs, and only the ones that survived it
# may be referenced by the strike.
kept = set(font.getGlyphOrder())
for subtable in strike.indexSubTables:
    subtable.names = [n for n in subtable.names if n in kept]
strike.indexSubTables = [s for s in strike.indexSubTables if s.names]
eblc.strikes = [strike]
ebdt.strikeData = [{n: b for n, b in bitmaps.items() if n in kept}]

# fontTools compiles the bitmap tables by glyph name, so attaching them to the
# subsetted font picks up the renumbered glyph ids.
font["EBLC"] = eblc
font["EBDT"] = ebdt

names = {1: FAMILY, 3: PS_NAME, 4: FAMILY, 6: PS_NAME, 16: FAMILY, 18: FAMILY}
name_table = font["name"]
for record in list(name_table.names):
    if record.nameID in names:
        name_table.setName(
            names[record.nameID],
            record.nameID,
            record.platformID,
            record.platEncID,
            record.langID,
        )

font.save(DST)

check = TTFont(DST)
assert "EBDT" in check and "EBLC" in check, "bitmap tables were lost"
size_table = check["EBLC"].strikes[0].bitmapSizeTable
print(
    f"{DST}: family={check['name'].getDebugName(1)!r} "
    f"ppem={size_table.ppemX} bitDepth={size_table.bitDepth} "
    f"glyphs={len(check.getGlyphOrder())}"
)
