## A list of unsupported SVG 1.1 features

For the list of unsupported SVG 2 features see: [svg2-changelog.md](./svg2-changelog.md)

### Elements

- Font based
  - `altGlyph`
  - `altGlyphDef`
  - `altGlyphItem`
  - `font-face-format`
  - `font-face-name`
  - `font-face-src`
  - `font-face-uri`
  - `font-face`
  - `font`
  - `glyph`
  - `glyphRef`
  - `hkern`
  - `missing-glyph`
  - `vkern`
- `color-profile`
- `use` with a reference to an external SVG file

### Attributes

- `clip` (deprecated in the SVG 2)
- `color-interpolation` outside gradient interpolation and luminance mask conversion
- `color-profile`
- `color-rendering`
- `direction`
- `font-size-adjust`
- `font-stretch`
- `glyph-orientation-horizontal` (removed in the SVG 2)
- `glyph-orientation-vertical` (deprecated in the SVG 2)
- `kerning` (removed in the SVG 2)
- `unicode-bidi`

**Note:** this list does not include elements and attributes outside the
[static SVG](http://www.w3.org/TR/SVG11/feature#SVG-static) subset.
