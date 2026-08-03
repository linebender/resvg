"""Build a small monochrome-bitmap test font from Terminus (TTF).

pyftsubset drops EBDT/EBLC, so the strike is pruned by hand and re-attached to
the subsetted font. The result is renamed because "Terminus" is an OFL Reserved
Font Name and this is a Modified Version.
"""

import sys
from fontTools.ttLib import TTFont
from fontTools.subset import Subsetter, Options

SRC = "TerminusTTF-Regular.ttf"
DST = "BitmapMono.subset.ttf"
PPEM = 16
FAMILY = "Bitmap Mono"
PS_NAME = "BitmapMono-Regular"

chars = [0x20] + list(range(0x30, 0x3A)) + list(range(0x41, 0x5B)) + list(range(0x61, 0x7B))

full = TTFont(SRC)
cmap = full.getBestCmap()
keep = {cmap[c] for c in chars if c in cmap} | {".notdef"}

# Pull the wanted strike out of the original font before subsetting drops it.
eblc, ebdt = full["EBLC"], full["EBDT"]
strike_idx = next(
    i for i, s in enumerate(eblc.strikes) if s.bitmapSizeTable.ppemX == PPEM
)
strike = eblc.strikes[strike_idx]
strike_data = {n: b for n, b in ebdt.strikeData[strike_idx].items() if n in keep}
for ist in strike.indexSubTables:
    ist.names = [n for n in ist.names if n in keep]
strike.indexSubTables = [ist for ist in strike.indexSubTables if ist.names]
eblc.strikes = [strike]
ebdt.strikeData = [strike_data]

subset = TTFont(SRC)
options = Options()
options.drop_tables += ["BDF ", "FFTM"]
subsetter = Subsetter(options=options)
subsetter.populate(glyphs=sorted(keep))
subsetter.subset(subset)

# fontTools compiles the bitmap tables by glyph name, so re-attaching them after
# the subset picks up the renumbered glyph ids.
eblc.strikes[0].indexSubTables = [
    ist for ist in eblc.strikes[0].indexSubTables
    if (setattr(ist, "names", [n for n in ist.names if n in subset.getGlyphOrder()]) or ist.names)
]
ebdt.strikeData = [{n: b for n, b in strike_data.items() if n in subset.getGlyphOrder()}]
subset["EBLC"] = eblc
subset["EBDT"] = ebdt

name = subset["name"]
for record in list(name.names):
    if record.nameID in (1, 3, 4, 6, 16, 18):
        value = {1: FAMILY, 3: PS_NAME, 4: FAMILY, 6: PS_NAME, 16: FAMILY, 18: FAMILY}[
            record.nameID
        ]
        name.setName(value, record.nameID, record.platformID, record.platEncID, record.langID)

subset.save(DST)

check = TTFont(DST)
assert "EBDT" in check and "EBLC" in check, "bitmap tables were lost"
size_table = check["EBLC"].strikes[0].bitmapSizeTable
print(f"{DST}: family={check['name'].getDebugName(1)!r} "
      f"ppem={size_table.ppemX} bitDepth={size_table.bitDepth} "
      f"glyphs={len(check.getGlyphOrder())}")
