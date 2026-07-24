# SVG 2 changelog

An attempt to list all changes between SVG 1.1 and SVG 2.

Somewhat similar to [Changes from SVG 1.1](https://www.w3.org/TR/SVG2/changes.html) from the SVG 2 spec, but actually lists all changes and not just changes to the spec itself. For example, that page doesn't list filter related changes and most of the text related changes are either omitted or scattered around the spec.

This document contains changes only to the static SVG subset. No animations, events and scripting.

A checkbox indicates that the related feature is implemented in `resvg`.

NOTE: This list is not final. This just things I was able to find so far. Patches are welcome.

## Data Types

### Added

- [x] A `turn` unit to [`<angle>`](https://www.w3.org/TR/css-values-3/#angles).
- [ ] Following units: `ch`, `rem`, `vw`, `vh`, `vmin`, `vmax` and `Q` to [`<length>`](https://www.w3.org/TR/css3-values/#lengths).
- [x] [`rgba()`](https://www.w3.org/TR/css-color-3/#rgba-color), [`hsl()`](https://www.w3.org/TR/css-color-3/#hsl-color) and [`hsla()`](https://www.w3.org/TR/css-color-3/#hsla-color) notations to [`<color>`](https://www.w3.org/TR/css-color-3/#colorunits).
- [x] A [`transparent`](https://www.w3.org/TR/css-color-3/#transparent) keyword to [`<color>`](https://www.w3.org/TR/css-color-3/#colorunits).
- [x] A `#RRGGBBAA` and `#RGBA` notation for colors. Part of [CSS Color 4](https://www.w3.org/TR/css-color-4/#hex-notation).
- [x] A [`opacity`](https://www.w3.org/TR/css-color-4/#transparency) property allows `<percentage>` now.

### Changed

- [x] [`<length>`](https://www.w3.org/TR/css3-values/#lengths) no longer includes the `%` unit. This variant was moved into a separate type: [`<length-percentage>`](https://www.w3.org/TR/css3-values/#typedef-length-percentage).
- [x] [`<FuncIRI>`](https://www.w3.org/TR/SVG11/filters.html#FilterProperty) was replaced with an [`<url>`](https://www.w3.org/TR/css3-values/#url-value). The main change here is that `<url>` allows quoted strings.

### Deprecated

- [CSS2 system colors](https://www.w3.org/TR/css-color-3/#css2-system).

### Quirks

- [`<color>`](https://www.w3.org/TR/css-color-3/#colorunits) includes an alpha value now, which should be accounted by `fill`, `stroke`, `flood-color` and `stop-color` properties. But not by `lighting-color` property. At least Chrome 92 and Firefox 91 doesn't do this.

<!-- ----------------------------------- -->

## Document Structure

### Added

- [ ] `refX` and `refY` [properties](https://www.w3.org/TR/SVG2/struct.html#SymbolAttributes) to the [`symbol`](https://www.w3.org/TR/SVG2/struct.html#SymbolElement) element.
- [x] An [`auto`](https://www.w3.org/TR/SVG2/geometry.html#Sizing) variant to [`image`](https://www.w3.org/TR/SVG2/embedded.html#ImageElement) element's `width` and `height` properties.
- [ ] A `lang` attribute. The same as `xml:lang`, but without the namespace.

### Changed

- [ ] `width` and `height` properties of the [`svg`](https://www.w3.org/TR/SVG2/struct.html#SVGElement) element are set to `auto` by default.

### Removed

- A `baseProfile` attribute from the [`svg`](https://www.w3.org/TR/SVG2/struct.html#SVGElement) element.
- A `version` attribute from the [`svg`](https://www.w3.org/TR/SVG2/struct.html#SVGElement) element.
- A `externalResourcesRequired` attribute.
- A `requiredFeatures` attribute.
- A `xml:base` attribute.

<!-- min-width and max-width ? -->

<!-- ----------------------------------- -->

## Styling

### Deprecated

- A [`clip`](https://www.w3.org/TR/css-masking-1/#clip-property) property.

<!-- ----------------------------------- -->

## Coordinate Systems, Transformations and Units

### Added

- [ ] A [`transform-box`](https://www.w3.org/TR/css-transforms-1/#transform-box) property.
- [x] A [`transform-origin`](https://www.w3.org/TR/css-transforms-1/#transform-origin-property) property.
- [ ] A [`vector-effect`](https://www.w3.org/TR/SVG2/coords.html#VectorEffects) property.

### Changed

- [ ] `transform`, `patternTransform` and `gradientTransform` are presentation attributes now. Which means that they can be resolved from CSS now.

### Removed

- A `defer` keyword from the [`preserveAspectRatio`](https://www.w3.org/TR/SVG2/coords.html#PreserveAspectRatioAttribute) attribute.

### Quirks

- CSS `transform` and SVG `transform` [have different syntax](https://www.w3.org/TR/css-transforms-1/#svg-syntax).

<!-- ----------------------------------- -->

## Basic Shapes

### Added

- [ ] A [`pathLength`](https://www.w3.org/TR/SVG2/paths.html#PathLengthAttribute) attribute to all [basic shapes](https://www.w3.org/TR/SVG2/shapes.html).

### Changed

- [x] `rx`/`ry` attributes on [`ellipse`](https://www.w3.org/TR/SVG2/shapes.html#EllipseElement) should be resolved using the same logic as [`rect`](https://www.w3.org/TR/SVG2/shapes.html#RectElement) uses.

<!-- ----------------------------------- -->

## Text

### Added

Basically everything from [CSS Text Module Level 3](https://www.w3.org/TR/css-text-3/).

- [ ] WOFF font support is required now.
- [x] A [`path`](https://www.w3.org/TR/SVG2/text.html#TextPathElementPathAttribute) property to [`textPath`](https://www.w3.org/TR/SVG2/text.html#TextPathElement).
- [ ] A [`side`](https://www.w3.org/TR/SVG2/text.html#TextPathElementSideAttribute) property to [`textPath`](https://www.w3.org/TR/SVG2/text.html#TextPathElement).
- [ ] A [`font-feature-settings`](https://www.w3.org/TR/css-fonts-3/#propdef-font-feature-settings) property.
- [x] A [`font-kerning`](https://www.w3.org/TR/css-fonts-3/#propdef-font-kerning) property.
- [ ] A [`font-synthesis`](https://www.w3.org/TR/css-fonts-3/#propdef-font-synthesis) property.
- [ ] A [`font-variant-caps`](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant-caps) property.
- [ ] A [`font-variant-east-asian`](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant-east-asian) property.
- [ ] A [`font-variant-ligatures`](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant-ligatures) property.
- [ ] A [`font-variant-numeric`](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant-numeric) property.
- [ ] A [`font-variant-position`](https://www.w3.org/TR/css-fonts-3/#propdef-font-variant-position) property.
- [ ] A [`line-height`](https://www.w3.org/TR/SVG2/text.html#LineHeightProperty) property.
- [ ] A [`text-align-last`](https://www.w3.org/TR/css-text-3/#propdef-text-align-last) property.
- [ ] A [`text-align`](https://www.w3.org/TR/css-text-3/#propdef-text-align) property.
- [ ] A [`text-indent`](https://www.w3.org/TR/css-text-3/#propdef-text-indent) property.
- [ ] A [`text-orientation`](https://www.w3.org/TR/css-writing-modes-3/#text-orientation) property.
- [ ] A [`text-overflow`](https://www.w3.org/TR/SVG2/text.html#TextOverflowProperty) property.
- [ ] A [`text-transform`](https://www.w3.org/TR/css-text-3/#text-transform-property) property.
- [ ] A [`unicode-range`](https://www.w3.org/TR/css-fonts-3/#descdef-unicode-range) property.
- [ ] A [`white-space`](https://www.w3.org/TR/SVG2/text.html#WhiteSpace) property.
- [ ] A [`text-decoration-line`](https://www.w3.org/TR/css-text-decor-3/#propdef-text-decoration-line) property.
- [ ] A [`text-decoration-style`](https://www.w3.org/TR/css-text-decor-3/#propdef-text-decoration-style) property.
- [ ] A [`text-decoration-color`](https://www.w3.org/TR/css-text-decor-3/#propdef-text-decoration-color) property.
- [ ] A [`text-underline-position`](https://www.w3.org/TR/css-text-decor-3/#propdef-text-underline-position) property.
- [ ] A [`text-decoration-fill`](https://www.w3.org/TR/SVG2/text.html#TextDecorationFillStroke) property.
- [ ] A [`text-decoration-stroke`](https://www.w3.org/TR/SVG2/text.html#TextDecorationFillStroke) property.
- [ ] A [`inline-size`](https://www.w3.org/TR/SVG2/text.html#InlineSize) property.
- [ ] A [`shape-inside`](https://www.w3.org/TR/SVG2/text.html#TextShapeInside) property.
- [ ] A [`shape-subtract`](https://www.w3.org/TR/SVG2/text.html#TextShapeSubtract) property.
- [ ] A [`shape-image-threshold`](https://www.w3.org/TR/SVG2/text.html#TextShapeImageThreshold) property.
- [ ] A [`shape-margin`](https://www.w3.org/TR/SVG2/text.html#TextShapeMargin) property.
- [ ] A [`shape-padding`](https://www.w3.org/TR/SVG2/text.html#TextShapePadding) property.
- [ ] New variants to [`font-variant`](https://drafts.csswg.org/css-fonts-3/#font-variant-prop) property. Previously it allowed only `small-caps`.
- [x] A `font-variant-css21` value to [`font`](https://www.w3.org/TR/css-fonts-3/#propdef-font) property.

<!-- text-emphasis ? -->
<!-- text-shadow ? -->

### Changed

- [x] [`textPath`](https://www.w3.org/TR/SVG2/text.html#TextPathElement) can reference [basic shapes](https://www.w3.org/TR/SVG2/shapes.html) now.
- [ ] Since CSS Fonts Module Level 4, the [`font-weight`](https://www.w3.org/TR/css-fonts-4/#font-weight-prop) property allows any value in a 1..1000 range.
- [x] A [`writing-mode`](https://www.w3.org/TR/SVG2/text.html#WritingModeProperty) property introduces the `horizontal-tb` and `vertical-lr` values from [CSS Writing Modes Level 3](https://www.w3.org/TR/css-writing-modes-3/#svg-writing-mode-css).
- [ ] [`dominant-baseline`](https://www.w3.org/TR/css-inline-3/#propdef-dominant-baseline) is inherited now.
- [ ] [`baseline-shift`](https://www.w3.org/TR/css-inline-3/#propdef-baseline-shift) is `0` by default, instead of `baseline`.
- [ ] Percentage values in a [`word-spacing`](https://www.w3.org/TR/css-text-3/#word-spacing-property) relate to a percentage of the affected character's width and not to viewport size now.
- [ ] `filter`, `clip-path`, `mask` and `opacity` properties can be set on `tspan` and `textPath` elements.
- [ ] A [`text-decoration`](https://www.w3.org/TR/css-text-decor-3/#propdef-text-decoration) property has a new, but backward compatible syntax.

### Removed

- A [`tref`](https://www.w3.org/TR/SVG11/text.html#TRefElement) element.
- A [`kerning`](https://www.w3.org/TR/SVG11/text.html#KerningProperty) property. Use [`font-kerning`](https://www.w3.org/TR/css-fonts-3/#font-kerning-prop) instead.
- A [`glyph-orientation-horizontal`](https://www.w3.org/TR/SVG11/text.html#GlyphOrientationHorizontalProperty) property.
- A [`altGlyph`](https://www.w3.org/TR/SVG11/text.html#AltGlyphElement) element.
- A [`altGlyphDef`](https://www.w3.org/TR/SVG11/text.html#AltGlyphDefElement) element.
- A [`altGlyphItem`](https://www.w3.org/TR/SVG11/text.html#AltGlyphItemElement) element.
- A [`glyphRef`](https://www.w3.org/TR/SVG11/text.html#GlyphRefElement) element.
- `reset-size`, `use-script` and `no-change` variants from [`dominant-baseline`](https://www.w3.org/TR/css-inline-3/#propdef-dominant-baseline).
- `auto`, `before-edge`, and `after-edge` variants from [`alignment-baseline`](https://www.w3.org/TR/css-inline-3/#propdef-alignment-baseline).
- `baseline` variant from [`baseline-shift`](https://www.w3.org/TR/css-inline-3/#propdef-baseline-shift).
- Percentage values from [`letter-spacing`](https://www.w3.org/TR/css-text-3/#letter-spacing-property).

### Deprecated

- A [`xml:space`](https://www.w3.org/TR/SVG11/struct.html#XMLSpaceAttribute) property.
- A [`glyph-orientation-vertical`](https://www.w3.org/TR/SVG2/text.html#GlyphOrientationVerticalProperty) property.
- A [`baseline-shift`](https://www.w3.org/TR/SVG2/text.html#BaselineShiftProperty) property. Use [`vertical-align`](https://drafts.csswg.org/css-inline/#transverse-alignment) instead.

### Quirks

- As of 2021, only Inkscape has [Text layout – Content Area](https://www.w3.org/TR/SVG2/text.html#TextLayoutContentArea) support, but still a very minimal one.
- `text-transform` is technically a CSS 2.1 property, but was not allowed in SVG 1.1

<!-- ----------------------------------- -->

## Painting

### Added

- [ ] An `arcs` variant to the [`stroke-linejoin`](https://www.w3.org/TR/SVG2/painting.html#LineJoin) property.
- [x] A `miter-clip` variant to the [`stroke-linejoin`](https://www.w3.org/TR/SVG2/painting.html#LineJoin) property.
- [x] (partial support) A [`paint-order`](https://www.w3.org/TR/SVG2/painting.html#PaintOrder) property.
- [x] `context-fill` and `context-stroke` variants to the [`<paint>`](https://www.w3.org/TR/SVG2/painting.html#SpecifyingPaint) type.
- [x] A [`mix-blend-mode`](https://www.w3.org/TR/compositing-1/#mix-blend-mode) property.
- [x] An [`isolation`](https://www.w3.org/TR/compositing-1/#isolation) property.
- [ ] `left`, `center` and `right` variants to `refX` and `refY` properties of the [`marker`](https://www.w3.org/TR/SVG2/painting.html#MarkerElement) element.
- [x] An `auto-start-reverse` variant to [`orient`](https://www.w3.org/TR/SVG2/painting.html#OrientAttribute) property of the [`marker`](https://www.w3.org/TR/SVG2/painting.html#MarkerElement) element
- [x] The `image-rendering` can appear as a presentation attribute with additional possible values. Currently, there is only best-effort support for "pixelated".
### Changed

- [x] Markers can be set on all shapes and not only on `path`.

### Quirks

- As of 2021, no one supports `stroke-linejoin:arcs`.

<!-- ----------------------------------- -->

## Gradients and Patterns

### Added

- [ ] A [`fr`](https://www.w3.org/TR/SVG2/pservers.html#RadialGradientElementFRAttribute) attribute to the `radialGradient` element

<!-- ----------------------------------- -->

## Clipping, Masking and Compositing

### Added

- [ ] [`<basic-shape>`](https://www.w3.org/TR/css-shapes-1/#typedef-basic-shape) and [`<geometry-box>`](https://www.w3.org/TR/css-masking-1/#typedef-geometry-box) variants to the [`clip-path`](https://www.w3.org/TR/css-masking-1/#the-clip-path) property.
- [ ] A [`mask-image`](https://www.w3.org/TR/css-masking-1/#the-mask-image) property.
- [ ] A [`mask-mode`](https://www.w3.org/TR/css-masking-1/#the-mask-mode) property.
- [ ] A [`mask-position`](https://www.w3.org/TR/css-masking-1/#the-mask-position) property.
- [ ] A [`mask-clip`](https://www.w3.org/TR/css-masking-1/#the-mask-clip) property.
- [ ] A [`mask-origin`](https://www.w3.org/TR/css-masking-1/#the-mask-origin) property.
- [ ] A [`mask-size`](https://www.w3.org/TR/css-masking-1/#the-mask-size) property.
- [ ] A [`mask-composite`](https://www.w3.org/TR/css-masking-1/#the-mask-composite) property.
- [x] A [`mask-type`](https://www.w3.org/TR/css-masking-1/#the-mask-type) property.
- [ ] A [`mask-border-source`](https://www.w3.org/TR/css-masking-1/#the-mask-border-source) property.
- [ ] A [`mask-border-mode`](https://www.w3.org/TR/css-masking-1/#the-mask-border-mode) property.
- [ ] A [`mask-border-slice`](https://www.w3.org/TR/css-masking-1/#the-mask-border-slice) property.
- [ ] A [`mask-border-width`](https://www.w3.org/TR/css-masking-1/#the-mask-border-width) property.
- [ ] A [`mask-border-outset`](https://www.w3.org/TR/css-masking-1/#the-mask-border-outset) property.
- [ ] A [`mask-border-repeat`](https://www.w3.org/TR/css-masking-1/#the-mask-border-repeat) property.
- [ ] A [`mask-border`](https://www.w3.org/TR/css-masking-1/#the-mask-border) property.

### Changed

- [ ] A [`mask`](https://www.w3.org/TR/css-masking-1/#the-mask) property has [a new grammar](https://www.w3.org/TR/css-masking-1/#typedef-mask-layer), backward compatible with SVG 1.1 one.
- [ ] An element can have multiple masks now.

<!-- ----------------------------------- -->

## Filter Effects

### Added

- [x] A [`feDropShadow`](https://www.w3.org/TR/filter-effects-1/#feDropShadowElement) element.
- [ ] An [`edgeMode`](https://www.w3.org/TR/filter-effects-1/#element-attrdef-fegaussianblur-edgemode) attribute to `feGaussianBlur` element.
- [x] [Filter functions](https://www.w3.org/TR/filter-effects-1/#filter-functions).
- [x] New [blend modes](https://www.w3.org/TR/compositing-1/#ltblendmodegt) to [`feBlend`](https://www.w3.org/TR/filter-effects-1/#feBlendElement) element.
- [ ] A [`no-composite`](https://www.w3.org/TR/filter-effects-1/#element-attrdef-feblend-no-composite) property to [`feBlend`](https://www.w3.org/TR/filter-effects-1/#feBlendElement) element.

### Changed

- [x] A `filter` property type changed from [`<FuncIRI>`](https://www.w3.org/TR/SVG11/filters.html#FilterProperty) to [`<filter-value-list>`](https://www.w3.org/TR/filter-effects-1/#typedef-filter-value-list).
- [ ] The [`saturate`](https://www.w3.org/TR/filter-effects-1/#element-attrdef-fecolormatrix-values) type in `feColorMatrix` can be larger than 1 now.

### Deprecated

- [x] An [`enable-background`](https://www.w3.org/TR/filter-effects-1/#AccessBackgroundImage) property.

### Quirks

- [Filter functions](https://www.w3.org/TR/filter-effects-1/#filter-functions) doesn't have a [filter region](https://www.w3.org/TR/filter-effects-1/#filter-region). Which means `blur()` and `drop-shadow()` cannot be losslessly converted to `filter` element. We have to manually calculate a new region (somehow).
- [Filter functions](https://www.w3.org/TR/filter-effects-1/#filter-functions) are always in sRGB color space, unlike a `filter` element, which is in linearRGB by default.

<!-- ----------------------------------- -->

## Linking

### Deprecated

- `xlink:href` in favor of `href`.

<!-- ----------------------------------- -->

## Fonts

### Removed

- A `font` element.
- A `glyph` element.
- A `missing-glyph` element.
- A `hkern` element.
- A `vkern` element.
- A `font-face` element.
- A `font-face-src` element.
- A `font-face-uri` element.
- A `font-face-format` element.
- A `font-face-name` element.
