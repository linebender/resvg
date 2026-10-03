"""Build small monochrome-bitmap test fonts from Terminus (TTF).

Two are produced: one with outlines behind its strikes, and an outline-free
sibling. A bitmap font need not carry outlines at all, and the two take
different paths through the renderer, so both have to be covered.

pyftsubset drops EBDT/EBLC, so the strike is pruned by hand and re-attached to
the subsetted font. The result is renamed because "Terminus" is an OFL Reserved
Font Name and this is a Modified Version.

Run next to TerminusTTF-Regular.ttf.
"""

from fontTools.subset import Options, Subsetter
from fontTools.ttLib import TTFont

SRC = "TerminusTTF-Regular.ttf"
DST = "BitmapMono.subset.ttf"
DST_NO_OUTLINE = "BitmapMonoNoOutline.subset.ttf"
# 16 and 24 leave a size in between them that has to fall back to the outline.
# 14 is the one strike here whose advance the outline cannot express: Terminus
# is 8x14 at this size and 8x16 at the next, while a single `hmtx` advance can
# only scale, so it says 7 where the strike says 8.
PPEMS = (14, 16, 24)
FAMILY = "Bitmap Mono"
PS_NAME = "BitmapMono-Regular"
FAMILY_NO_OUTLINE = "Bitmap Mono No Outline"
PS_NAME_NO_OUTLINE = "BitmapMonoNoOutline-Regular"
# Space, digits and the latin alphabet.
CHARS = [0x20, *range(0x30, 0x3A), *range(0x41, 0x5B), *range(0x61, 0x7B)]

font = TTFont(SRC)
cmap = font.getBestCmap()
wanted = {cmap[c] for c in CHARS if c in cmap} | {".notdef"}

# The bitmap tables have to be taken from the original font, since subsetting
# drops them.
eblc, ebdt = font["EBLC"], font["EBDT"]
kept_strikes = [
    (s, ebdt.strikeData[i])
    for i, s in enumerate(eblc.strikes)
    if s.bitmapSizeTable.ppemX in PPEMS
]
assert len(kept_strikes) == len(PPEMS), "a requested strike is missing"

options = Options()
options.drop_tables += ["BDF ", "FFTM"]
subsetter = Subsetter(options=options)
subsetter.populate(glyphs=sorted(wanted))
subsetter.subset(font)

# Subsetting can pull in additional glyphs, and only the ones that survived it
# may be referenced by the strike.
kept = set(font.getGlyphOrder())
for strike, _ in kept_strikes:
    for subtable in strike.indexSubTables:
        subtable.names = [n for n in subtable.names if n in kept]
    strike.indexSubTables = [s for s in strike.indexSubTables if s.names]
eblc.strikes = [strike for strike, _ in kept_strikes]
ebdt.strikeData = [
    {n: b for n, b in bitmaps.items() if n in kept} for _, bitmaps in kept_strikes
]

# fontTools compiles the bitmap tables by glyph name, so attaching them to the
# subsetted font picks up the renumbered glyph ids.
font["EBLC"] = eblc
font["EBDT"] = ebdt

def rename(target, family, ps_name):
    labels = {1: family, 3: ps_name, 4: family, 6: ps_name, 16: family, 18: family}
    table = target["name"]
    for record in list(table.names):
        if record.nameID in labels:
            table.setName(
                labels[record.nameID],
                record.nameID,
                record.platformID,
                record.platEncID,
                record.langID,
            )


rename(font, FAMILY, PS_NAME)
font.save(DST)

# The outline-free sibling. Dropping the outline tables outright is what a real
# pixel font does; a font that keeps empty outlines instead is a different case,
# because the renderer then finds an outline and draws nothing.
bare = TTFont(DST)
for tag in ("glyf", "loca", "cvt ", "fpgm", "prep", "gasp"):
    if tag in bare:
        del bare[tag]
rename(bare, FAMILY_NO_OUTLINE, PS_NAME_NO_OUTLINE)
bare.save(DST_NO_OUTLINE)

check = TTFont(DST)
assert "EBDT" in check and "EBLC" in check, "bitmap tables were lost"
strikes = [
    (s.bitmapSizeTable.ppemX, s.bitmapSizeTable.bitDepth) for s in check["EBLC"].strikes
]
print(
    f"{DST}: family={check['name'].getDebugName(1)!r} "
    f"strikes={strikes} glyphs={len(check.getGlyphOrder())}"
)

bare_check = TTFont(DST_NO_OUTLINE)
assert "glyf" not in bare_check and "loca" not in bare_check, "outlines survived"
assert "EBDT" in bare_check and "EBLC" in bare_check, "bitmap tables were lost"
print(
    f"{DST_NO_OUTLINE}: family={bare_check['name'].getDebugName(1)!r} "
    f"strikes={[s.bitmapSizeTable.ppemX for s in bare_check['EBLC'].strikes]} "
    f"outlines=none"
)
