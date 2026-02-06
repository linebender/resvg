How fonts were subsetted:

Twitter Color Emoji
1. Download: https://github.com/13rac1/twemoji-color-font/releases/download/v14.0.2/TwitterColorEmoji-SVGinOT-14.0.2.zip
2. Run `fonttools subset TwitterColorEmoji-SVGinOT.ttf --unicodes="U+1F601,U+1F980,U+1F3F3,U+FE0F,U+200D,U+1F308,U+1F600,U+1F603,U+1F90C,U+1F90F" --output-file=TwitterColorEmoji.subset.ttf`

Noto Color Emoji (CBDT)
1. Download: https://github.com/googlefonts/noto-emoji/blob/main/fonts/NotoColorEmoji.ttf
2. Run `fonttools subset NotoColorEmoji.ttf --unicodes="U+1F600" --output-file=NotoColorEmojiCBDT.subset.ttf`

Noto COLOR Emoji (COLRv1)
1. Download: https://fonts.google.com/noto/specimen/Noto+Color+Emoji
2. Run `fonttools subset NotoColorEmoji-Regular.ttf --unicodes="U+1F436,U+1F41D,U+1F313,U+1F973" --output-file=NotoColorEmojiCOLR.subset.ttf`
3. Run `fonttools ttx NotoColorEmojiCOLR.subset.ttf`
4. Go to the <name> section and rename all instances of "Noto Color Emoji" to "Noto Color Emoji COLR" (so that
we can distinguish them from CBDT in tests).
5. Run `fonttools ttx -f NotoColorEmojiCOLR.subset.ttx`

Roboto Flex (Variable Font)
1. Download: https://github.com/googlefonts/roboto-flex/raw/main/fonts/RobotoFlex%5BGRAD%2CXOPQ%2CXTRA%2CYOPQ%2CYTAS%2CYTDE%2CYTFI%2CYTLC%2CYTUC%2Copsz%2Cslnt%2Cwdth%2Cwght%5D.ttf
2. Run `pyftsubset RobotoFlex*.ttf --unicodes="U+0020-007E" --layout-features='*' --output-file=RobotoFlex.subset.ttf`
3. Copy OFL license from https://github.com/googlefonts/roboto-flex/blob/main/OFL.txt

Source Han Sans HW SC (CJK)
1. Download: https://github.com/adobe-fonts/source-han-sans/releases/download/2.005R/14_SourceHanSansHWSC.zip
2. Run `pyftsubset SourceHanSansHWSC-Regular.otf --unicodes="U+4E2D,U+5165,U+5203,U+56FD,U+5E38,U+65E5,U+672C,U+6D77,U+76F4,U+89D2,U+9577,U+975E,U+9AA8,U+0020-007F,U+0100-017F,U+0600-06FF,U+3040-309F,U+30A0-30FF,U+3000-303F,U+FE30-FE4F,U+FF00-FFEF,U+2000-206F,U+AC00,U+0100-017F,U+1EA0-1EFF" --output-file=SourceHanSansHWSC-Regular.subset.ttf`