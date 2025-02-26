# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

This changelog also contains important changes in dependencies.

## [Unreleased]

## [0.45.0] - 2025-02-26
This is the first release under the stewardship of [Linebender][], who is now responsible for maintenance of this crate.
Many thanks to Yevhenii Reizner for the years of hard work that he has poured into this and other crates.

Please note that the license of this project changed from `MPL-2.0` to `Apache-2.0 OR MIT`.
See [resvg#838](https://github.com/linebender/resvg/issues/838) for more information.

This release has an MSRV of 1.65 for `usvg` and 1.67.1 for `resvg` and the C API.

### Added
- Support for the `background-color` attribute.
- Support for additional `image-rendering` attributes.
- Support for the `!important` CSS flag.
- Support for Luma JPEG images.
- (c-api) `resvg_options_set_stylesheet`.
  Thanks to [@michabay05][].
- (svgtypes) Support for floating point hue in `hsl()` and `hsla()`.

### Changed
- License to `Apache-2.0 OR MIT`.
  See [resvg#838](https://github.com/linebender/resvg/issues/838) for more information.
- Updated WebP decoder for bug fixes and improved performance.
  Thanks to [@Shnatsel][].
- MSRV of resvg and c-api bumped to 1.67.1.
- `fontdb` and `rustybuzz` have been updated.
- Updated other dependencies.
- (svgtypes) Simplified color component rounding and bounds checking.
- Improved handling of paths with paint order `markers` but no actual markers.

### Fixed
- Relative unit handling when `use` references `symbol`.
- (svgtypes) Rounding of hues in HSL to RGB conversion.
- (svgtypes) Rounding of alpha.

## [0.44.0] - 2024-09-28
### Added
- Stylesheet injection support.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- (c-api) `resvg_get_object_bbox`
- (c-api) `cargo-c` metadata.
  Thanks to [@lu-zero](https://github.com/lu-zero).
- Implement `From` for `fontdb` and `usvg` font types.
  Thanks to [@dhardy](https://github.com/dhardy).

### Changed
- (c-api) `resvg_get_image_bbox` returns a _layer_ and not _object_ bounding box now.
  Use `resvg_get_object_bbox` to preserve the old behavior.

### Fixed
- (svgtypes) Path parsing with `S` or `T` segments after `A`.
- Bounding box calculation for the root group used for `viewBox` flattening.

## [0.43.0] - 2024-08-10
### Added
- Support WebP images.
  Thanks to [@notjosh](https://github.com/notjosh).

### Changed
- Use `zune-jpeg` instead of `jpeg-decoder`.
  Thanks to [@mattfbacon](https://github.com/mattfbacon).
- Update dependencies.

### Fixed
- Canvas size limits calculation.
- SVG fonts handling.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- Transforms in COLR fonts.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).

## [0.42.0] - 2024-06-01
### Added
- `resvg` can render color fonts now, aka Emojis.<br>
  In TrueType terms, `COLRv0`, `COLRv1` (mostly), `sbix`, `CBDT` and `SVG` tables are supported.<br>
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- Fonts matching and fallback can be controlled by the caller via `usvg::FontResolver` now.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- `usvg::Options::font_resolver`. Similar to `usvg::Options::image_href_resolver` we already had.
- `usvg::Options::fontdb`
- Support double-quoted FuncIRIs, aka `url("#id")`.
- `image` element viewbox flattening.<br>
  Instead of having `usvg::Image::view_box` that the caller should handle themselves,
  we instead replace it with `transform` and optional `clip-path`.
  This greatly simplifies `image` rendering.
- `usvg::Image::size`
- Tree viewbox flattening.<br>
  Similar to `image` above, but affects the root `svg` element instead.
- `pattern` viewbox flattening.<br>
  Similar to `image` above, but for patterns.
- Improve vertical text rendering.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).

### Changed
- `usvg::fontdb::Database` should be set in `usvg::Options` and not passed
  to the parser separately now.
- `usvg::Options` and `usvg::ImageHrefResolver` have a lifetime now.
- Replace `usvg::Visibility` enum with just `bool`.
- `usvg::Path::visibility()` is replaced with `usvg::Path::is_visible()`
- `usvg::Image::visibility()` is replaced with `usvg::Image::is_visible()`
- `usvg::TextSpan::visibility()` is replaced with `usvg::TextSpan::is_visible()`
- Always represent `feImage` content as a link to an element.<br>
  In SVG, `feImage` can contain a link to an element or a base64 image data, just like `image`.
  From now, the inlined base64 data will always be represented by a link to an actual `image` element.
  ```xml
  <filter>
    <feImage xlink:href="data:image/png;base64,..."/>
  </filter>
  ```
  will be parsed as
  ```xml
  <image id="image1" xlink:href="data:image/png;base64,..."/>
  <filter>
    <feImage xlink:href="#image1"/>
  </filter>
  ```
  This simplifies `feImage` rendering, since we don't have to handle both cases now.
- The `--list-fonts` resvg argument can be used without providing an SVG file now.
  Can simply call `resvg --list-fonts` now.
- The `--list-fonts` resvg argument includes generic font family names as well now.
- Make sure all warning and errors are printed to stderr.
  Thanks to [@ahaoboy](https://github.com/ahaoboy).

### Removed
- `usvg::ViewBox`, `usvg::AspectRatio`, `usvg::Align` types. Nol longer used.
- `usvg::filter::Image::aspect`. No longer needed.
- `usvg::filter::Image::rendering_mode`. No longer needed.
- `usvg::filter::Image::data`. Use `usvg::filter::Image::root` instead.
- `usvg::Tree::view_box`. No longer needed.
- `usvg::Image::view_box`. No longer needed.
- `usvg::Image::pattern`. No longer needed.
- `usvg::utils::align_pos`. No longer needed.
- `usvg::Visibility`. No longer needed.
- (c-api) `resvg_get_image_viewbox`. Use `resvg_get_image_size` instead.

### Fixed
- `context-fill` handling.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).

## [0.41.0] - 2024-04-03
### Added
- `context-fill` and `context-stroke` support.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- `usvg::Text::layouted()`, which returns a list of glyph IDs.
  It can be used to manually draw glyphs, unlike with `usvg::Text::flattened()`, which returns
  just vector paths.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).

### Fixed
- Missing text when a `text` element uses multiple fonts and one of them produces ligatures.
- Absolute transform propagation during `use` resolving.
- Absolute transform propagation during nested `svg` resolving.
- `Node::abs_transform` documentation. The current element's transform _is_ included.

## [0.40.0] - 2024-02-17
### Added
- `usvg::Tree` is `Send + Sync` compatible now.
- `usvg::WriteOptions::preserve_text` to control how `usvg` generates an SVG.
- `usvg::Image::abs_bounding_box`

### Changed
- All types in `usvg` are immutable now. Meaning that `usvg::Tree` cannot be modified
  after creation anymore.
- All struct fields in `usvg` are private now. Use getters instead.
- All `usvg::Tree` parsing methods require the `fontdb` argument now.
- All `defs` children like gradients, patterns, clipPaths, masks and filters are guarantee
  to have a unique, non-empty ID.
- All `defs` children like gradients, patterns, clipPaths, masks and filters are guarantee
  to have `userSpaceOnUse` units now. No `objectBoundingBox` units anymore.
- `usvg::Mask` is allowed to have no children now.
- Text nodes will not be parsed when the `text` build feature isn't enabled.
- `usvg::Tree::clip_paths`, `usvg::Tree::masks`, `usvg::Tree::filters` returns
  a pre-collected slice of unique nodes now.
  It's no longer a closure and you do not have to deduplicate nodes by yourself.
- `usvg::filter::Primitive::x`, `y`, `width` and `height` methods were replaced
  with `usvg::filter::Primitive::rect`.
- Split `usvg::Tree::paint_servers` into `usvg::Tree::linear_gradients`,
  `usvg::Tree::radial_gradients`, `usvg::Tree::patterns`.
  All three returns pre-collected slices now.
- A `usvg::Path` no longer can have an invalid bbox. Paths with an invalid bbox will be
  rejected during parsing.
- All `usvg` methods that return bounding boxes return non-optional `Rect` now.
  No `NonZeroRect` as well.
- `usvg::Text::flattened` returns `&Group` and not `Option<&Group>` now.
- `usvg::ImageHrefDataResolverFn` and `usvg::ImageHrefStringResolverFn`
  require `fontdb::Database` argument.
- All shared nodes are stored in `Arc` and not `Rc<RefCell>` now.
- `resvg::render_node` now includes filters bounding box. Meaning that a node with a blur filter
  no longer be clipped.
- Replace `usvg::utils::view_box_to_transform` with `usvg::ViewBox::to_transform`.
- Rename `usvg::XmlOptions` into `usvg::WriteOptions` and embed `xmlwriter::Options`.

### Removed
- `usvg::Tree::postprocess()` and `usvg::PostProcessingSteps`. No longer needed.
- `usvg::ClipPath::units()`, `usvg::Mask::units()`, `usvg::Mask::content_units()`,
  `usvg::Filter::units()`, `usvg::Filter::content_units()`, `usvg::LinearGradient::units()`,
  `usvg::RadialGradient::units()`, `usvg::Pattern::units()`, `usvg::Pattern::content_units()`
  and `usvg::Paint::units()`. They are always `userSpaceOnUse` now.
- `usvg::Units`. No longer needed.

### Fixed
- Text bounding box is accounted during SVG size resolving.
  Previously, only paths and images were included.
- Font selection when an italic font isn't explicitly marked as one.
- Preserve `image` aspect ratio when only `width` or `height` are present.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).

## [0.39.0] - 2024-02-06
### Added
- `font` shorthand parsing.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- `usvg::Group::abs_bounding_box`
- `usvg::Group::abs_stroke_bounding_box`
- `usvg::Path::abs_bounding_box`
- `usvg::Path::abs_stroke_bounding_box`
- `usvg::Text::abs_bounding_box`
- `usvg::Text::abs_stroke_bounding_box`

### Changed
- All `usvg-*` crates merged into one. There is just the `usvg` crate now, as before.

### Removed
- `usvg::Group::abs_bounding_box()` method. It's a field now.
- `usvg::Group::abs_filters_bounding_box()`
- `usvg::TreeParsing`, `usvg::TreePostProc` and `usvg::TreeWriting` traits.
  They are no longer needed.

### Fixed
- `font-family` parsing.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- Absolute bounding box calculation for paths.

## [0.38.0] - 2024-01-21
### Added
- Each `usvg::Node` stores its absolute transform now.
  `Node::abs_transform()` executes in constant time now.
- `usvg::Tree::calculate_bounding_boxes` to calculate all bounding boxes beforehand.
- `usvg::Node::bounding_box` which returns a precalculated node's bounding box in object coordinates.
- `usvg::Node::abs_bounding_box` which returns a precalculated node's bounding box in canvas coordinates.
- `usvg::Node::stroke_bounding_box` which returns a precalculated node's bounding box,
  including stroke, in object coordinates.
- `usvg::Node::abs_stroke_bounding_box` which returns a precalculated node's bounding box,
  including stroke, in canvas coordinates.
- (c-api) `resvg_get_node_stroke_bbox`
- `usvg::Node::filters_bounding_box`
- `usvg::Node::abs_filters_bounding_box`
- `usvg::Tree::postprocess`

### Changed
- `resvg` renders `usvg::Tree` directly again. `resvg::Tree` is gone.
- `usvg` no longer uses `rctree` for the nodes tree implementation.
  The tree is a regular `enum` now.
  - A caller no longer need to use the awkward `*node.borrow()`.
  - No more panics on incorrect mutable `Rc<RefCell>` access.
  - Tree nodes respect tree's mutability rules. Before, one could mutate tree nodes when the tree
    itself is not mutable. Because `Rc<RefCell>` provides a shared mutable access.
- Filters, clip paths, masks and patterns are stored as `Rc<RefCell<T>>` instead of `Rc<T>`.
  This is required for proper mutability since `Node` itself is no longer an `Rc`.
- Rename `usvg::NodeKind` into `usvg::Node`.
- Upgrade to Rust 2021 edition.

### Removed
- `resvg::Tree`. No longer needed. `resvg` can render `usvg::Tree` directly once again.
- `rctree::Node` methods. The `Node` API is completely different now.
- `usvg::NodeExt`. No longer needed.
- `usvg::Node::calculate_bbox`. Use `usvg::Node::abs_bounding_box` instead.
- `usvg::Tree::convert_text`. Use `usvg::Tree::postprocess` instead.
- `usvg::TreeTextToPath` trait. No longer needed.

### Fixed
- Mark `mask-type` as a presentation attribute.
- Do not show needless warnings when parsing some attributes.
- `feImage` rendering with a non-default position.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).

## [0.37.0] - 2023-12-16
### Added
- `usvg` can write text back to SVG now.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- `--preserve-text` flag to the `usvg` CLI tool.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- Support [`transform-origin`](https://drafts.csswg.org/css-transforms/#transform-origin-property)
  property.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- Support non-default markers order via
  [`paint-order`](https://svgwg.org/svg2-draft/painting.html#PaintOrder).
  Previously, only fill and stroke could have been swapped.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- `usvg_tree::Text::flattened` that will contain a flattened/outlined text.
- `usvg_tree::Text::bounding_box`. Will be set only after text flattening.
- Optimize `usvg_tree::NodeExt::abs_transform` by storing absolute transforms in the tree
  instead of calculating them each time.

### Changed
- `usvg_tree::Text::positions` was replaced with `usvg_tree::Text::dx` and `usvg_tree::Text::dy`.<br>
  `usvg_tree::CharacterPosition::x` and `usvg_tree::CharacterPosition::y` are gone.
  They were redundant and you should use `usvg_tree::TextChunk::x`
  and `usvg_tree::TextChunk::y` instead.
- `usvg_tree::LinearGradient::id` and `usvg_tree::RadialGradient::id` are moved to
  `usvg_tree::BaseGradient::id`.
- Do not generate element IDs during parsing. Previously, some elements like `clipPath`s
  and `filter`s could have generated IDs, but it wasn't very reliable and mostly unnecessary.
  Renderer doesn't rely on them and usvg writer would generate them anyway.
- Text-to-paths conversion via `usvg_text_layout::Tree::convert_text` no longer replaces
  original text elements with paths, but instead puts them into `usvg_tree::Text::flattened`.

### Removed
- The `transform` field from `usvg_tree::Path`, `usvg_tree::Image` and `usvg_tree::Text`.
  Only `usvg_tree::Group` can have it.<br>
  It doesn't break anything, because those properties were never used before anyway.<br>
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- `usvg_tree::CharacterPosition`
- `usvg_tree::Path::text_bbox`. Use `usvg_tree::Text::bounding_box` instead.
- `usvg_text_layout::TextToPath` trait for `Text` nodes.
  Only the whole tree can be converted at once.

### Fixed
- Path object bounding box calculation. We were using point bounds instead of tight contour bounds.
  Was broken since v0.34
- Convert text-to-paths in embedded SVGs as well. The one inside the `Image` node.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- Indirect `text-decoration` resolving in some cases.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- (usvg) Clip paths writing to SVG.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).

## [0.36.0] - 2023-10-01
### Added
- `stroke-linejoin=miter-clip` support. SVG2.
  Thanks to [@torokati44](https://github.com/torokati44).
- Quoted FuncIRI support. Like `fill="url('#gradient')"`. SVG2.
  Thanks to [@romanzes](https://github.com/romanzes).
- Allow float values in `rgb()` and `rgba()` colors. SVG2.
  Thanks to [@yisibl](https://github.com/yisibl).
- `auto-start-reverse` variant support to `orient` in markers. SVG2.
  Thanks to [@EpicEricEE](https://github.com/EpicEricEE).

### Changed
- Update dependencies.

### Fixed
- Increase precision of the zero-scale transform check.
  Was rejecting some valid transforms before.
- Panic when rendering a very specific text.
- Greatly improve parsing performance when an SVG has a lot of references.
  Thanks to [@wez](https://github.com/wez).
- (Qt API) Fix scaling factor calculation.
  Thanks to [@missdeer](https://github.com/missdeer).

## [0.35.0] - 2023-06-27
### Fixed
- Panic when an element is completely outside the viewbox.

### Removed
- `FillPaint` and `StrokePaint` filter inputs support.
  It's a mostly undocumented SVG feature that no one supports and no one uses.
  And it was adding a significant complexity to the codebase.
- `usvg::filter::Filter::fill_paint` and `usvg::filter::Filter::stroke_paint`.
- `BackgroundImage`, `BackgroundAlpha`, `FillPaint` and `StrokePaint` from `usvg::filter::Input`.
- `usvg::Group::filter_fill_paint` and `usvg::Group::filter_stroke_paint`.

## [0.34.1] - 2023-05-28
### Fixed
- Transform components order. Affects only `usvg` SVG output and C API.

## [0.34.0] - 2023-05-27
### Changed
- `usvg` uses `tiny-skia` geometry primitives now, including the `Path` container.<br>
  The main difference compared to the old `usvg` primitives
  is that `tiny-skia` uses `f32` instead of `f64`.
  So while in theory we could loose some precision, in practice, `f32` is used mainly
  as a storage type and precise math operations are still done using `f64`.<br>
  `tiny-skia` primitives are move robust, strict and have a nicer API.<br>
  More importantly, this change reduces the peak memory usages for SVGs with large paths
  (in terms of the number of segments).
  And removes the need to convert `usvg::PathData` into `tiny-skia::Path` before rendering.
  Which was just a useless reallocation.
- All numbers are stored as `f32` instead of `f64` now.
- Because we use `tiny-skia::Path` now, we allow _quadratic curves_ as well.
  This includes `usvg` CLI output.
- Because we allow _quadratic curves_ now, text might render slightly differently (better?).
  This is because TrueType fonts contain only _quadratic curves_
  and we were converting them to cubic before.
- `usvg::Path` no longer implements `Default`. Use `usvg::Path::new` instead.
- Replace `usvg::Rect` with `tiny_skia::NonZeroRect`.
- Replace `usvg::PathBbox` with `tiny_skia::Rect`.
- Unlike the old `usvg::PathBbox`, `tiny_skia::Rect` allows both width and height to be zero.
  This is not an error.
- `usvg::filter::Turbulence::base_frequency` was split into `base_frequency_x` and `base_frequency_y`.
- `usvg::NodeExt::calculate_bbox` no longer includes stroke bbox.
- (c-api) Use `float` instead of `double` everywhere.
- The `svgfilters` crate was merged into `resvg`.
- The `rosvgtree` crate was merged into `usvg-parser`.
- `usvg::Group::filter_fill` moved to `usvg::filter::Filter::fill_paint`.
- `usvg::Group::filter_stroke` moved to `usvg::filter::Filter::stroke_paint`.

### Remove
- `usvg::Point`. Use `tiny_skia::Point` instead.
- `usvg::FuzzyEq`. Use `usvg::ApproxEqUlps` instead.
- `usvg::FuzzyZero`. Use `usvg::ApproxZeroUlps` instead.
- (c-api) `resvg_path_bbox`. Use `resvg_rect` instead.
- `svgfilters` crate.
- `rosvgtree` crate.

### Fixed
- Write `transform` on `clipPath` children in `usvg` SVG output.
- Do not duplicate marker children IDs.
  Previously, each element resolved for a marker would preserve its ID.
  Affects only `usvg` SVG output and doesn't affect rendering.

## [0.33.0] - 2023-05-17
### Added
- A new rendering algorithm.<br>
  When rendering [isolated groups](https://razrfalcon.github.io/notes-on-svg-parsing/isolated-groups.html),
  aka layers, we have to know the layer bounding box beforehand, which is ridiculously hard in SVG.<br>
  Previously, resvg would simply use the canvas size for all the layers.
  Meaning that to render a 10x10px layer on a 1000x1000px canvas, we would have to allocate and then blend
  a 1000x1000px layer, which is just a waste of CPU cycles.<br>
  The new rendering algorithm is able to calculate layer bounding boxes, which dramatically improves
  performance when rendering a lot of tiny layers on a large canvas.<br>
  Moreover, it makes performance more linear with a canvas size increase.<br>
  The [paris-30k.svg](https://github.com/google/forma/blob/681e8bfd348caa61aab47437e7d857764c2ce522/assets/svgs/paris-30k.svg)
  sample from [google/forma](https://github.com/google/forma) is rendered _115 times_ faster on M1 Pro now.
  From ~33760ms down to ~290ms. 5269x3593px canvas.<br>
  If we restrict the canvas to 1000x1000px, which would contain only the actual `paris-30k.svg` content,
  then we're _13 times_ faster. From ~3252ms down to ~253ms.
- `resvg::Tree`, aka a render tree, which is an even simpler version of `usvg::Tree`.
  `usvg::Tree` had to be converted into `resvg::Tree` before rendering now.

### Changed
- Restructure the root directory. All crates are in the `crates` directory now.
- Restructure tests. New directory structure and naming scheme.
- Use `resvg::Tree::render` instead of `resvg::render`.
- resvg's `--export-area-drawing` option uses calculated bounds instead of trimming
  excessive alpha now. It's faster, but can lead to a slightly different output.
- (c-api) Removed `fit_to` argument from `resvg_render`.
- (c-api) Removed `fit_to` argument from `resvg_render_node`.
- `usvg::ScreenSize` moved to `resvg`.
- `usvg::ScreenRect` moved to `resvg`.
- Rename `resvg::ScreenSize` into `resvg::IntSize`.
- Rename `resvg::ScreenRect` into `resvg::IntRect`.

### Removed
- `filter` build feature from `resvg`. Filters are always enabled now.
- `resvg::FitTo`
- `usvg::utils::view_box_to_transform_with_clip`
- `usvg::Size::to_screen_size`. Use `resvg::IntSize::from_usvg` instead.
- `usvg::Rect::to_screen_size`. Use `resvg::IntSize::from_usvg(rect.size())` instead.
- `usvg::Rect::to_screen_rect`. Use `resvg::IntRect::from_usvg` instead.
- (c-api) `resvg_fit_to`
- (c-api) `resvg_fit_to_type`

### Fixed
- Double quotes parsing in `font-family`.

## [0.32.0] - 2023-04-23
### Added
- Clipping and masking is up to 20% faster.
- `mask-type` property support. SVG2
- `usvg_tree::MaskType`
- `usvg_tree::Mask::kind`
- (rosvgtree) New SVG 2 mask attributes.

### Changed
- `BackgroundImage` and `BackgroundAlpha` filter inputs will produce the same output
  as `SourceGraphic` and `SourceAlpha` respectively.

### Removed
- `enable-background` support. This feature was never supported by browsers
  and was deprecated in SVG 2. To my knowledge, only Batik has a good support of it.
  Also, it's a performance nightmare, which caused multiple issues in resvg already.
- `usvg_tree::EnableBackground`
- `usvg_tree::Group::enable_background`
- `usvg_tree::NodeExt::filter_background_start_node`

### Fixed
- Improve rectangular clipping anti-aliasing quality.
- Mask's RGB to Luminance converter was ignoring premultiplied alpha.

## [0.31.1] - 2023-04-22
### Fixed
- Use the latest `tiny-skia` to fix SVGs with large masks rendering.

## [0.31.0] - 2023-04-10
### Added
- `usvg::Tree::paint_servers`
- `usvg::Tree::clip_paths`
- `usvg::Tree::masks`
- `usvg::Tree::filters`
- `usvg::Node::subroots`
- (usvg) `--coordinates-precision` and `--transforms-precision` writing options.
  Thanks to [@flxzt](https://github.com/flxzt).

### Fixed
- `fill-opacity` and `stroke-opacity` resolving.
- Double `transform` when resolving `symbol`.
- `symbol` clipping when its viewbox is the same as the document one.
- (usvg) Deeply nested gradients, patterns, clip paths, masks and filters
  were ignored during SVG writing.
- Missing text in nested clip paths and mask, text decoration patterns, filter inputs and feImage.

## [0.30.0] - 2023-03-25
### Added
- Readd `usvg` CLI tool. Can be installed via cargo as before.

### Changed
- Extract most `usvg` internals into new `usvg-tree` and `usvg-parser` crates.
  `usvg-tree` contains just the SVG tree and all the types.
  `usvg-parser` parsers SVG into `usvg-tree`.
  And `usvg` is just an umbrella crate now.
- To use `usvg::Tree::from*` methods one should import the `usvg::TreeParsing` trait now.
- No need to import `usvg-text-layout` manually anymore. It is part of `usvg` now.
- `rosvgtree` no longer reexports `svgtypes`.
- `rosvgtree::Node::attribute` returns just a string now.
- `rosvgtree::Node::find_attribute` returns just a `rosvgtree::Node` now.
- Rename `usvg::Stretch` into `usvg::FontStretch`.
- Rename `usvg::Style` into `usvg::FontStyle`.
- `usvg::FitTo` moved to `resvg::FitTo`.
- `usvg::IsDefault` trait is private now.

### Removed
- `rosvgtree::FromValue`. Due to Rust's orphan rules this trait is pretty useless.

### Fixed
- Recursive markers detection.
- Skip malformed `transform` attributes without skipping the whole element.
- Clipping path rectangle calculation for nested `svg` elements.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- Panic when applying `text-decoration` on text with only one cluster.
  Thanks to [@LaurenzV](https://github.com/LaurenzV).
- (Qt API) Image size wasn't initialized. Thanks to [@missdeer](https://github.com/missdeer).
- `resvg` CLI allows files with XML DTD again.
- (svgtypes) Handle implicit MoveTo after ClosePath segments.

## [0.29.0] - 2023-02-04
### Added
- `resvg` CLI loads system fonts only when an input SVG has text nodes now.
  Fonts loading is an IO-heavy operation and by avoiding it we can speed up `resvg` execution.
- `usvg::Group::should_isolate`
- `usvg::Tree::has_text_nodes`

### Changed
- Some `usvg` internals were moved into the new `rosvgtree` crate.
- Dummy groups are no longer removed. Use `usvg::Group::should_isolate` to check
  if a group affects rendering.
- `usvg-text-layout::TreeTextToPath::convert_text` no longer has the `keep_named_groups` argument.
- MSRV bumped to 1.65
- Update dependencies.

### Removed
- `usvg::Options::keep_named_groups`. Dummy groups are no longer removed.
- (c-api) `resvg_options_set_keep_named_groups`
- (Qt API) `ResvgOptions::setKeepNamedGroups`

### Fixed
- Missing `font-family` handling.
- `font-weight` resolving.

## [0.28.0] - 2022-12-03
### Added
- `usvg::Text` and `usvg::NodeKind::Text`.

### Changed
- `usvg` isn't converting text to paths by default now. A caller must call
  `usvg::Tree::convert_text` or `usvg::Text::convert` from `usvg-text-layout` crate on demand.
- `usvg` text layout implementation moved into `usvg-text-layout` crate.
- During SVG size recovery, when no `width`, `height` and `viewBox` attributes have been set,
  text nodes are no longer taken into an account. This is because a text node has no bbox
  before conversion into path(s), which we no longer doing during parsing.
- `usvg` is purely an SVG parser now. It doesn't convert text to paths
  and doesn't write SVG anymore.
- `usvg::filter::ConvolveMatrixData` methods are fields now.

### Removed
- `usvg` CLI binary. No alternatives for now.
- All `usvg` build features.
  - `filter`. Filter elements are always parsed by `usvg` now.
  - `text`. Text elements are always parsed by `usvg` now.
  - `export`. `usvg` cannot write an SVG anymore.
- `usvg::Tree::to_string`. `usvg` cannot write an SVG anymore.
- `usvg::TransformFromBBox` trait. This is just a regular `usvg::Transform` method now.
- `usvg::OptionsRef`. `usvg::Options` is enough from now.
- `usvg::Options::fontdb`. Used only by `usvg-text-layout` now.
- `--dump-svg` from `resvg`.

## [0.27.0] - 2022-11-27
### Added
- `lengthAdjust` and `textLength` attributes support.
- Support automatic `image` size detection.
  `width` and `height` attributes can be omitted or set to `auto` on `image` now. SVG2

### Fixed
- `--query-all` flag in `resvg` CLI.
- Percentage values resolving.

## [0.26.1] - 2022-11-21
### Fixed
- Allow `dominant-baseline` and `alignment-baseline` to be set via CSS.

## [0.26.0] - 2022-11-20
### Added
- Minimal `dominant-baseline` and `alignment-baseline` support.
- `mix-blend-mode` and `isolation` support. SVG2
- Allow writing resvg output to stdout.
- Allow disabling text kerning using `kerning="0"` and `style="font-kerning:none"`. SVG2
- Allow `<percentage>` values for `opacity`, `fill-opacity`, `stroke-opacity`,
  `flood-opacity` and `stop-opacity` attributes.<br>
  You can write `opacity="50%"` now. SVG2

### Changed
- Disable focal point correction on radial gradients to conform with SVG 2. SVG2
- Update `feMorphology` radius value resolving.

### Fixed
- Do not clip nested `svg` when only the `viewBox` attribute is present.

## [0.25.0] - 2022-10-30
### Added
- Partial `paint-order` attribute support.
  Markers can only be under or above the shape.

### Fixed
- Compilation issues caused by `rustybuzz` update.

## [0.24.0] - 2022-10-22
### Added
- CSS3 `writing-mode` variants `vertical-rl` and `vertical-lr`.
  Thanks to [yisibl](https://github.com/yisibl).
- (tiny-skia) AArch64 Neon SIMD support. Up to 3x faster on Apple M1.

### Changed
- `usvg::Tree` stores only `Group`, `Path` and `Image` nodes now.
  Instead of emulating an SVG file structure, where gradients, patterns, filters, clips and masks
  are part of the nodes tree (usually inside the `defs` element), we reference them using `Rc`
  from now.
  This change makes `usvg` a bit simpler. Makes `usvg` API way easier, since instead of
  looking for a node via `usvg::Tree::defs_by_id` the caller can access the type directly via `Rc`.
  And makes creation of custom `usvg::Tree`s way easier.
- `clip_path`, `mask` and `filters` `usvg::Group` fields store `Rc` instead of `String` now.
- `usvg::NodeExt::units` was moved to `usvg::Paint::units`.
- `usvg::filter::ImageKind::Use` stores `usvg::Node` instead of `String`.
- `usvg::PathData` stores commands and points separately now to reduce overall memory usage.
- `usvg::PathData` segments should be accessed via `segments()` now.
- Most numeric types have been moved to the `strict-num` crate.
- Rename `NormalizedValue` into `NormalizedF64`.
- Rename `PositiveNumber` into `PositiveF64`.
- Raw number of numeric types should be accessed via `get()` method instead of `value()` now.
- `usvg::TextSpan::font_size` is `NonZeroPositiveF64` instead of `f64` now.
- Re-export `usvg` and `tiny-skia` dependencies in `resvg`.
- Re-export `roxmltree` dependency in `usvg`.
- (usvg) Output float precision is reduced from 11 to 8 digits.

### Removed
- `usvg::Tree::create`. `usvg::Tree` is an open struct now.
- `usvg::Tree::root`. It's a public field now.
- `usvg::Tree::svg_node`. Replaced with `usvg::Tree` public fields.
- `defs`, `is_in_defs`, `append_to_defs` and `defs_by_id` from `usvg::Tree`.
  We no longer emulate SVG structure. No alternative.
- `usvg::Tree::is_in_defs`. There are no `defs` anymore.
- `usvg::Paint::Link`. We store gradient and patterns directly in `usvg::Paint` now.
- `usvg::Svg`. No longer needed. `size` and `view_box` are `usvg::Tree` fields now.
- `usvg::SubPathIter` and `usvg::PathData::subpaths`. No longer used.

### Fixed
- Path bbox calculation scales stroke width too.
  Thanks to [growler](https://github.com/growler).
- (tiny-skia) Round caps roundness.
- (xmlparser) Stack overflow on specific files.
- (c-api) `resvg_is_image_empty` output was inverted.

## [0.23.0] - 2022-06-11
### Added
- `#RRGGBBAA` and `#RGBA` color notation support.
  Thanks to [demurgos](https://github.com/demurgos).

### Fixed
- Panic during recursive `pattern` resolving.
  Thanks to [FylmTM](https://github.com/FylmTM).
- Spurious warning when using `--export-id`.
  Thanks to [benoit-pierre](https://github.com/benoit-pierre).

## [0.22.0] - 2022-02-20
### Added
- Support `svg` referenced by `use`. External SVG files are still not supported.

### Changed
- `ttf-parser`, `fontdb` and `rustybuzz` have been updated.

## [0.21.0] - 2022-02-13
### Added
- `usvg::ImageHrefResolver` that allows a custom `xlink:href` handling.
  Thanks to [antmelnyk](https://github.com/antmelnyk).
- `usvg::Options::image_href_resolver`
- Support for GIF images inside the `<image>` element.
- (fontdb) Support for loading user fonts on Windows.
- (fontdb) Support for parsing fontconfig config files on Linux.
  For now, only to retrieve a list of font dirs.

### Changed
- MSRV bumped to 1.51
- `usvg::ImageKind` stores data as `Arc<Vec<u8>>` and not just `Vec<u8>` now.

### Fixed
- Every nested `svg` element defines a new viewBox now. Previously, we were always using the root one.
- Correctly handle SVG size calculation when SVG doesn't have a size and any elements.
- Improve groups ungrouping speed.

## [0.20.0] - 2021-12-29
### Changed
- `resvg::render` and `resvg::render_node` accept a transform now.
- (c-api) `resvg_render` and `resvg_render_node` accept a transform now.
- `usvg::Color` is a custom type and not a `svgtypes::Color` reexport now.
- `usvg::Color` doesn't contain alpha anymore, which have been added in v0.16
  Alpha would be automatically flattened.
  This makes [Micro SVG](https://github.com/linebender/resvg/blob/main/crates/usvg/docs/spec.adoc)
  compatible with SVG 1.1 again.
- (c-api) Rename `RESVG_FIT_TO_*` into `RESVG_FIT_TO_TYPE_*`.

### Fixed
- The `--background` argument in `resvg` correctly handles alpha now.
- Fix building usvg without filter feature but with export.

## [0.19.0] - 2021-10-04
### Added
- Better text-on-path converter accuracy by accounting the current transform.

### Changed
- `usvg::NodeExt::abs_transform` includes current node transform now.
- Improved turbulence filter performance. Thanks to [akindle](https://github.com/akindle).
- Multiple dependencies updated.

## [0.18.0] - 2021-09-12
### Added
- `filter` build feature. Enabled by default.
- `usvg::PathBbox` and `resvg_path_bbox` (to C API).

### Changed
- (usvg) All filter related types are under the `filter` module now.
- (usvg) Remove `Fe` prefix from all filter types.
- (c-api) `resvg_get_node_bbox` returns `resvg_path_bbox` now.

### Fixed
- Horizontal and vertical lines processing.
- C API building without the `text` feature.

## [0.17.0] - 2021-09-04
### Added
- `tiny-skia` updated with support of images larger than 8000x8000 pixels.
- `feDropShadow` support. SVG2
- [`<filter-value-list>`](https://www.w3.org/TR/filter-effects-1/#typedef-filter-value-list) support.
  Meaning that the `filter` attribute can have multiple values now.
  Like `url(#filter1) blur(2)`. SVG2
- All [filter functions](https://www.w3.org/TR/filter-effects-1/#filter-functions). SVG2
- Support all [new](https://www.w3.org/TR/compositing-1/#ltblendmodegt) `feBlend` modes. SVG2
- Automatic SVG size detection when `width`/`height`/`viewBox` is not set.
  Thanks to [reknih](https://github.com/reknih).
- `usvg::Options::default_size`
- `--default-width` and `--default-height` to usvg.

### Changed
- `usvg::Group::filter` is a list of filter IDs now.
- `usvg::FeColorMatrixKind::Saturate` accepts any positive `f64` value now.
- `svgfilters::ColorMatrix::Saturate` accepts any positive `f64` value now.
- Fonts memory mapping was split into a separate build feature: `memmap-fonts`.
  Now you can build resvg/usvg with `system-fonts`, but without `memmap-fonts`.
  Enabled by default.
- The `--dump-svg` argument in resvg CLI tool should be enabled using `--features dump-svg` now.
  No enabled by default.
- `usvg::Tree::to_string` is behind the `export` build feature now.

### Fixed
- When writing SVG, `usvg` will use `rgba()` notations for colors instead of `#RRGGBB`.

## [0.16.0] - 2021-08-22
### Added
- CSS3 colors support. Specifically `rgba`, `hsl`, `hsla` and `transparent`. SVG2
- Allow missing `rx`/`ry` attributes on `ellipse`. SVG2
- Allow markers on all shapes. SVG2
- `textPath` can reference basic shapes now. SVG2
- `usvg::OptionsRef`, which is a non-owned `usvg::Options` variant.
- `simplecss` updated with CSS specificity support.
- `turn` angle unit support. SVG2
- Basic `font-variant=small-caps` support. No font fallback.
- `--export-area-page` to resvg.
- `--export-area-drawing` to resvg.

### Changed
- `resvg::render_node` requires `usvg::Tree` now.
- `usvg::Color` gained an `alpha` field.

### Removed
- `usvg::Node::tree`. Cannot be implemented efficiently anymore.
- `usvg::SystemFontDB`. No longer needed.

### Fixed
- `pattern` scaling.
- Greatly improve `symbol` resolving speed in `usvg`.
- Whitespaces trimming on nested `tspan`.

## [0.15.0] - 2021-06-13
### Added
- Allow reading SVG from stdin in `resvg` binary.
- `--id-prefix` to `usvg`.
- `FitTo::Size`
- `resvg` binary accepts `--width` and `--height` args together now.
  Previously, only `--width` or `--height` were allowed.
- `usvg::Path::text_bbox`
- The maximum number of SVG elements is limited by 1_000_000 now.
  Mainly to prevent a billion laugh style attacks.
- The maximum SVG elements nesting is limited by 1024 now.
- `usvg::Error::ElementsLimitReached`

### Changed
- Improve clipping and masking performance on large images.
- Remove layers caching. This was a pointless optimization.
- Split _Preprocessing_ into _Reading_ and _Parsing_ in `resvg --perf`.
- `usvg::XmlOptions` rewritten.
- `usvg::Tree::to_string` requires a reference to `XmlOptions` now.

### Removed
- `usvg::Tree::from_file`. Use `from_data` or `from_str` instead.
- `usvg::Error::InvalidFileSuffix`
- `usvg::Error::FileOpenFailed`
- (c-api) `RESVG_ERROR_INVALID_FILE_SUFFIX`

### Fixed
- Ignore tiny blur values. It could lead to a transparent image.
- `use` style propagation when used with `symbol`.
- Vertical text layout with relative offsets.
- Text bbox calculation. `usvg` uses font metrics instead of path bbox now.

## [0.14.1] - 2021-04-18
### Added
- Allow `href` without the `xlink` namespace.
  This feature is part of SVG 2 (which we do not support),
  but there are more and more files like this in the wild.

### Changed
- (usvg) Do not write `usvg:version` to the output SVG.

### Fixed
- (usvg) `overflow='inherit'` resolving.
- (usvg) SVG Path length calculation that affects `startOffset` property in `textPath`.
- (usvg) Fix `feImage` resolving when the linked element has
  `opacity`, `clip-path`, `mask` and/or `filter` attributes.
- (usvg) Fix chained `feImage` resolving.
- CLI arguments processing.

## [0.14.0] - 2021-03-06
### Fixed
- Multiple critical bugs in `tiny-skia`.

## [0.13.1] - 2021-01-20
### Fixed
- `image` with float size scaling.
- Critical bug in `tiny-skia`.

## [0.13.0] - 2020-12-21
### Added
- `--resources-dir` option to CLI tools.
- (usvg) `Tree::from_xmltree`

### Changed
- Remove the `Image` struct. `render()` and `render_node()` methods now accept `tiny_skia::PixmapMut`.
- Update `fontdb`.
- Update `tiny-skia`.
- (c-api) `resvg_size` uses `double` instead of `uint32_t` now.
- (qt-api) `defaultSize()` and `defaultSizeF()` methods now return SVG size
  and not SVG viewbox size.
- (usvg) `Options::path` changed to `Options::resources_dir` and requires a directory now.
- (c-api) `resvg_options_set_file_path` changed to `resvg_options_set_resources_dir`
  and requires a directory now.
- (qt-api) `ResvgOptions::setFilePath` changed to `ResvgOptions::setResourcesDir`
  and requires a directory now.

### Fixed
- Support multiple values inside a `text-decoration` attribute.

### Removed
- `Image`. Use `tiny_skia::PixmapMut` instead.
- (c-api) `resvg_image` struct and `resvg_image_*` methods. `resvg` renders onto
  the provided buffer now.
- (c-api) `resvg_color`, because unused.

## [0.12.0] - 2020-12-05
### Changed
- resvg no longer requires a C++ compiler!
- `tiny-skia` was updated to a pure Rust version, which means that `resvg` no longer
  depends on `clang` and should work on 32bit targets.
- `rustybuzz` was updated to a pure Rust version.
- `tools/explorer-thumbnailer` is back and written in Rust now.
  Thanks to [gentoo90](https://github.com/gentoo90).

### Fixed
- (usvg) Do not panic when a font has a zero-sized underline thickness.
- (usvg) Multiple `textPath` processing fixes by [chubei-oppen](https://github.com/chubei-oppen).
- (qt-api) `boundsOnElement` and `boundingBox` were returning transposed bounds.

## [0.11.0] - 2020-07-04
### Highlights
- All backends except Skia were removed. Skia is the only official one from now.
- New C API implementation.

### Added
- Support for user-defined fonts in usvg, resvg and C API.
- `--serif-family`, `--sans-serif-family`, `--cursive-family`, `--fantasy-family`
  `--monospace-family`, `--use-font-file`, `--use-fonts-dir`, `--skip-system-fonts` and `--list-fonts`
  options to all CLI tools.
- New tests suite. Instead of testing against the previous build, now we're testing against
  prerendered PNG images. Which is way faster.<br>
  And you can test resvg without the internet connection now.<br>
  And all you need is just `cargo test`.

### Changed
- Library uses an embedded Skia by default now.
- Switch `harfbuzz_rs` with `rustybuzz`.
- Rendering doesn't require `usvg::Options` now.
- (usvg) The `fontdb` module moved into its own crate.
- (usvg) `fontconfig` is no longer used for matching
  [generic fonts](https://www.w3.org/TR/2018/REC-css-fonts-3-20180920/#generic-family-value)
  on Linux. Mainly because it's very slow.
- (usvg) When an `image` element contains a file path, the file will be loaded into memory now,
  instead of simply storing a file path. And will be dumped as base64 on SVG save.
  In case of an SVG image, it will be loaded as a `Tree` and saved as base64 encoded XML on save.
- (usvg) `ImageData` replaced with `ImageKind`.
- (usvg) Fonts database is empty by default now and should be filled manually.
- (c-api) Almost a complete rewrite.

### Removed
- All backends except the Skia one.
- `Options` from all backends. We don't use it anymore.
- (usvg) `ImageFormat`.
- (c-api) Rendering on a backends canvas no longer supported. Was constantly misused.

## [0.10.0] - 2020-06-19

### Changed
- The `resvg` crate has been split into four: resvg-cairo, resvg-qt, resvg-skia and resvg-raqote.<br/>
  So from now, instead of enabling a required backend via cargo features,
  you should select a required backend-specific crate.<br/>
  This allows us to have a better integration with a selected 2D library.<br/>
  And we also have separated C API implementations now.<br/>
  And each backend has its own vendored archive too.
- (qt-backend) Use `QImage` instead of Rust libraries for raster images loading.

### Removed
- The `resvg` crate. Use backend-specific crates.
- `tools/rendersvg`. Each backend has its own CLI tool now.
- `tools/usvg`. `usvg` implements CLI by default now.
- (c-api) `resvg_*_render_to_file` methods.
- (qt-backend) `jpeg-decoder` and `png` dependencies.

## [0.9.1] - 2020-06-03
### Fixed
- Stack overflow when `enable-background` and `filter` are set on the same element.
- Grayscale PNG loading.
- Allow building on BSD.
- (usvg) Font fallback when shaping produces a different amount of glyphs.
- (usvg) Ignore a space after the last character during `letter-spacing` processing.
- (usvg) `marker-end` rendering when the last segment is a curve with the second control point
  that coincides with end point.
- (usvg) Accept embedded `image` data without mime.
- (usvg) Fonts search in a home directory on Linux.
- (usvg) `dy` calculation for `textPath` thanks to [Stoeoef](https://github.com/Stoeoef)
- (usvg) `textPath` resolving when a referenced path has a transform.<br/>
  Thanks to [Stoeoef](https://github.com/Stoeoef).
- (usvg) Load user fonts on macOS too.
- (xmlparser) Parsing comment before DTD.

## [0.9.0] - 2020-01-18
### Added
- `feConvolveMatrix`, `feMorphology`, `feDisplacementMap`, `feTurbulence`,
  `feDiffuseLighting` and `feSpecularLighting` support.
- `BackgroundImage`, `BackgroundAlpha`, `FillPaint` and `StrokePaint` support as a filter input.
- Load grayscale raster images.
- `enable-background` support.
- resvg/usvg can be built without text rendering support now.
- `OutputImage::make_vec` and `OutputImage::make_rgba_vec`.
- `feImage` with a reference to an internal element.

### Changed
- `feComposite` k1-4 coefficients can have any number now.
  This matches browsers behaviour.
- Use `flate2` instead of `libflate` for GZip decoding.
- (usvg) `fill` and `stroke` attributes will always be set for `path` now.
- (usvg) `g`, `path` and `image` can now be set inside `defs`. Required by `feImage`.
- (c-api) Rename `resvg_*_render_to_image` into `resvg_*_render_to_file`.

### Fixed
- (usvg) Transform processing during text-to-path conversion.
- `feComposite` with fully transparent region was producing an invalid result.
- Fallback to `matrix` in `feColorMatrix` when `type` is not set or invalid.
- ID preserving for `use` elements.
- `feFlood` with subregion and `primitiveUnits=objectBoundingBox`.
- (harfbuzz_rs) Memory leak.

## [0.8.0] - 2019-08-17
### Added
- A [Skia](https://skia.org/) backend thanks to
  [JaFenix](https://github.com/JaFenix).
- `feComponentTransfer` support.
- `feColorMatrix` support.
- A better CSS support.
- An `*.otf` fonts support.
- (usvg) `dx`, `dy` are supported inside `textPath` now.
- Use a box blur for `feGaussianBlur` with `stdDeviation`>=2.
  This is 4-8 times faster than IIR blur.
  Thanks to [Shnatsel](https://github.com/Shnatsel).

### Changed
- All backends are using Rust crates for raster images loading now.
- Use `pico-args` instead of `gumdrop` to reduced the build time of `tools/rendersvg`
  and `tools/usvg`.
- (usvg) The `xmlwriter` is used for SVG generation now.
  Almost 2x faster than generating an `svgdom`.
- (usvg) Optimize font database initialization. Almost 50% faster.
- Use a lower PNG compression ratio to speed up PNG generation.
  Depending on a backend and image can be 2-4x faster.
- `OutputImage::save` -> `OutputImage::save_png`.
- (usvg) `Path::segments` -> `Path::data`.
- Cairo backend compilation is 2x faster now due to overall changes.
- Performance improvements (Oxygen Icon theme SVG-to-PNG):
  - cairo-backend: 22% faster
  - qt-backend: 20% faster
  - raqote-backend: 34% faster

### Fixed
- (qt-api) A default font resolving.
- (usvg) `baseline-shift` processing inside `textPath`.
- (usvg) Remove all `tref` element children.
- (usvg) `tref` with `xml:space` resolving.
- (usvg) Ignore nested `tref`.
- (usvg) Ignore invalid `clipPath` children that were referenced via `use`.
- (usvg) `currentColor` will always fallback to black now.
  Previously, `stroke` was set to `none` which is incorrect.
- (usvg) `use` can reference an element inside a non-SVG element now.
- (usvg) Collect all styles for generic fonts and not only *Regular*.
- (usvg) Parse only presentation attributes from the `style` element and attribute.

### Removed
- (cairo-backend) `gdk-pixbuf` dependency.
- (qt-backend) JPEG image format plugin dependency.
- `svgdom` dependency.

## [0.7.0] - 2019-06-19
### Added
- New text layout implementation:
  - `textPath` support.
  - `writing-mode` support, aka vertical text.
  - [Text BIDI reordering](http://www.unicode.org/reports/tr9/).
  - Better text shaping.
  - `word-spacing` is supported for all backends now.
  - [`harfbuzz`](https://github.com/harfbuzz/harfbuzz) dependency.
  - Subscript, superscript offsets are extracted from font and not hardcoded now.
- `shape-rendering`, `text-rendering` and `image-rendering` support.
- The `arithmetic` operator for `feComposite`.
- (usvg) `--quiet` argument.
- (c-api) `resvg_get_image_bbox`.
- (qt-api) `ResvgRenderer::boundingBox`.
- (resvg) A [raqote](https://github.com/jrmuizel/raqote) backend thanks to
  [jrmuizel](https://github.com/jrmuizel). Still experimental.

### Changed
- Text will be converted into paths on the `usvg` side now.
- (resvg) Do not rescale images before rendering. This is faster and better.
- (usvg) An `image` element with a zero or negative size will be skipped now.
  Previously, a linked image size was used, which is incorrect.
- Geometry primitives (`Rect`, `Size`, etc) are immutable and always valid now.
- (usvg) The default `color-interpolation-filters` attribute will not be exported now.

### Removed
- (usvg) All text related structures and enums. Text will be converted into `Path` now.
- `InitObject` and `init()` because they are no longer needed.
- (c-api) `resvg_handle`, `resvg_init`, `resvg_destroy`.
- (c-api) `resvg_cairo_get_node_bbox` and `resvg_qt_get_node_bbox`.
  Use backend-independent `resvg_get_node_bbox` instead.
- (cairo-backend) `pango` dependency.
- (resvg) `Backend::calc_node_bbox`. Use `Node::calculate_bbox()` instead.

### Fixed
- `letter-spacing` on cursive scripts (like Arabic).
- (rctree) Prevent stack overflow on a huge, deeply nested SVG.
- (c-api) `resvg_is_image_empty` was always returning `false`.
- (resvg) Panic when `filter` with `objectBoundingBox` was set on an empty group.
- (usvg) `mask` with `objectBoundingBox` resolving.
- (usvg) `pattern`'s `viewBox` attribute resolving via `href`.
- (roxmltree) Namespace resolving.

## [0.6.1] - 2019-03-16
### Fixed
- (usvg) `transform` multiplication.
- (usvg) `use` inside `clipPath` resolving.

## [0.6.0] - 2019-03-16
### Added
- Nested `baseline-shift` support.
- (qt-api) `renderToImage`.
- (usvg) A better algorithm for unused defs (`defs` element children, like gradients) removal.
- (usvg) `Error::InvalidSize`.
- (c-api) `RESVG_ERROR_INVALID_SIZE`.

### Changed
- (usvg) A major rewrite.
- `baseline-shift` with `sub`, `super` and percent values calculation.
- Marker resolving moved completely to `usvg`.
- If an SVG doesn't have a valid size than an error will occur.
  Previously, an empty tree was produced.
- (qt-api) `render` methods are `const` now.
- (usvg) Disable default attributes exporting.

### Removed
- (usvg) Marker element and attributes. Markers will be resolved just like `use` now.

### Fixed
- (resvg) During the `tspan` rendering, the `text` bbox will be used instead
  of the `tspan` bbox itself. This is the correct behaviour by the SVG spec.
- (cairo-backend) `font-family` parsing.
- (usvg) `filter:none` processing.
- (usvg) `text` inside `text` processing.
- (usvg) Endless loop during `use` resolving.
- (usvg) Endless loop when SVG has indirect recursive `xlink:href` links.
- (usvg) Endless loop when SVG has recursive `marker-*` links.
- (usvg) Panic during `use` resolving.
- (usvg) Panic during inherited attributes resolving.
- (usvg) Groups regrouping.
- (usvg) `dx`/`dy` processing on `text`.
- (usvg) `textAnchor` resolving.
- (usvg) Ignore `fill-rule` on `text`.
- (svgtypes) Style with comments parsing.
- (roxmltree) Namespaces resolving.

## [0.5.0] - 2019-01-04
### Added
- `marker` support.
- Partial `baseline-shift` support.
- `letter-spacing` support.
- (qt-backend) `word-spacing` support.
  Does not work on the cairo backend.
- tools/explorer-thumbnailer
- tools/kde-dolphin-thumbnailer

### Fixed
- Object bounding box calculation.
- Pattern scaling.
- Nested `objectBoundingBox` support.
- (usvg) `color` on `use` resolving.
- (usvg) `offset` attribute resolving inside the `stop` element.
- (usvg) Ungrouping of groups with non-inheritable attributes.
- (usvg) `rotate` attribute resolving.
- (usvg) Paths without stroke and fill will no longer be removed.
  Required for a proper bbox resolving.
- (usvg) Coordinates resolving when units are `userSpaceOnUse`.
- (usvg) Groups regrouping. Caused an incorrect rendering of `clipPath`
  that had `filter` on a child.
- (usvg) Style attributes resolving on the root `svg` element.
- (usvg) `SmoothCurveTo` and `SmoothQuadratic` conversion.
- (usvg) `symbol` resolving.
- (cairo-backend) Font ascent calculation.
- (qt-backend) Stroking of LineTo specified as CurveTo.
- (svgdom) `stroke-miterlimit` attribute parsing.
- (svgdom) `length` and `number` attribute types parsing.
- (svgdom) `offset` attribute parsing.
- (svgdom) IRI resolving order when SVG has duplicated ID's.

## [0.4.0] - 2018-12-13
### Added
- (resvg) Initial filters support.
- (resvg) Nested `clipPath` and `mask` support.
- (resvg) MSVC support.
- (rendersvg) `font-family`, `font-size` and `languages` to args.
- (usvg) `systemLanguage` attribute support.
- (usvg) Default font family and size is configurable now.
- (c-api) `RESVG_ERROR_PARSING_FAILED`.
- (c-api) `font_family`, `font_size` and `languages` to `resvg_options`.
- (qt-api) `ResvgRenderer::setDevicePixelRatio`.

### Changed
- (rendersvg) Use `gumdrop` instead of `getopts`.
- (c-api) Qt wrapper is header-only now.

### Fixed
- (cairo-backend) Text layout.
- (cairo-backend) Rendering of a zero length subpath with a square cap.
- (qt-backend) Transform retrieving via Qt bindings.
- (resvg) Recursive SVG images via `image` tag.
- (resvg) Bbox calculation of the text with rotate.
- (resvg) Invisible elements processing.
- (qt-api) SVG from QByteArray loading when data is invalid.
- (usvg) `display` attribute processing.
- (usvg) Recursive `mask` resolving.
- (usvg) `inherit` attribute value resolving.
- (svgdom) XML namespaces resolving.

### Removed
- (rendersvg) `failure` dependency.

## [0.3.0] - 2018-05-23
### Added
- (c-api) `resvg_is_image_empty`.
- (c-api) `resvg_error` enum.
- (c-api) Qt wrapper.
- (resvg) Advanced text layout support (lists of x, y, dx, dy and rotate).
- (resvg) SVG support for `image` element.
- (usvg) `symbol` element support.
- (usvg) Nested `svg` elements support.
- (usvg) Paint fallback resolving.
- (usvg) Bbox validation for shapes that use painting servers.
- (svgdom) Elements from ENTITY resolving.

### Changed
- (c-api) `resvg_parse_tree_from_file`, `resvg_parse_tree_from_data`
  `resvg_cairo_render_to_image` and `resvg_qt_render_to_image`
  will return an error code now.
- (cairo-backend) Use `gdk-pixbuf` crate instead of `image`.
- (resvg) `Render::render_to_image` and `Render::render_node_to_image` will return
  `Option` and not `Result` now.
- (resvg) New geometry primitives implementation.
- (resvg) Rename `render_*` modules to `backend_`.
- (rendersvg) Use `getopts` instead of `clap` to reduce the executable size.
- (svgtypes) `StreamExt::parse_iri` and `StreamExt::parse_func_iri` will parse
  not only well-formed data now.

### Fixed
- (qt-backend) Gradient with `objectBoundingBox` rendering.
- (qt-backend) Text bounding box detection during the rendering.
- (cairo-backend) `image` element clipping.
- (cairo-backend) Layers management.
- (c-api) `resvg_get_node_transform` will return a correct transform now.
- (resvg) `text-decoration` thickness.
- (resvg) `pattern` scaling.
- (resvg) `image` without size rendering.
- (usvg) Panic during `visibility` resolving.
- (usvg) Gradients with one stop resolving.
- (usvg) `use` attributes resolving.
- (usvg) `clipPath` and `mask` attributes resolving.
- (usvg) `offset` attribute in `stop` element resolving.
- (usvg) Incorrect `font-size` attribute resolving.
- (usvg) Gradient stops resolving.
- (usvg) `switch` element resolving.
- (svgdom) Mixed `xml:space` processing.
- (svgtypes) `Paint::from_span` poor performance.

### Removed
- (c-api) `resvg_error_msg_destroy`.
- (resvg) `parse_rtree_*` methods. Use `usvg::Tree::from_` instead.
- (resvg) `Error`.

## [0.2.0] - 2018-04-24
### Added
- (svg) Partial `clipPath` support.
- (svg) Partial `mask` support.
- (svg) Partial `pattern` support.
- (svg) `preserveAspectRatio` support.
- (svg) Check that an external image is PNG or JPEG.
- (rendersvg) Added `--query-all` and `--export-id` arguments to render SVG items by ID.
- (rendersvg) Added `--perf` argument for a simple performance stats.

### Changed
- (resvg) API is completely new.

### Fixed
- `font-size` attribute inheritance during `use` resolving.

[Linebender]: https://github.com/linebender
[@michabay05]: https://github.com/michabay05
[@Shnatsel]: https://github.com/Shnatsel

[Unreleased]: https://github.com/linebender/resvg/compare/v0.45.0...HEAD
[0.45.0]: https://github.com/linebender/resvg/compare/v0.44.0...v0.45.0
[0.44.0]: https://github.com/linebender/resvg/compare/v0.43.0...v0.44.0
[0.43.0]: https://github.com/linebender/resvg/compare/v0.42.0...v0.43.0
[0.42.0]: https://github.com/linebender/resvg/compare/v0.41.0...v0.42.0
[0.41.0]: https://github.com/linebender/resvg/compare/v0.40.0...v0.41.0
[0.40.0]: https://github.com/linebender/resvg/compare/v0.39.0...v0.40.0
[0.39.0]: https://github.com/linebender/resvg/compare/v0.38.0...v0.39.0
[0.38.0]: https://github.com/linebender/resvg/compare/v0.37.0...v0.38.0
[0.37.0]: https://github.com/linebender/resvg/compare/v0.36.0...v0.37.0
[0.36.0]: https://github.com/linebender/resvg/compare/v0.35.0...v0.36.0
[0.35.0]: https://github.com/linebender/resvg/compare/v0.34.1...v0.35.0
[0.34.1]: https://github.com/linebender/resvg/compare/v0.34.0...v0.34.1
[0.34.0]: https://github.com/linebender/resvg/compare/v0.33.0...v0.34.0
[0.33.0]: https://github.com/linebender/resvg/compare/v0.32.0...v0.33.0
[0.32.0]: https://github.com/linebender/resvg/compare/v0.31.1...v0.32.0
[0.31.1]: https://github.com/linebender/resvg/compare/v0.31.0...v0.31.1
[0.31.0]: https://github.com/linebender/resvg/compare/v0.30.0...v0.31.0
[0.30.0]: https://github.com/linebender/resvg/compare/v0.29.0...v0.30.0
[0.29.0]: https://github.com/linebender/resvg/compare/v0.28.0...v0.29.0
[0.28.0]: https://github.com/linebender/resvg/compare/v0.27.0...v0.28.0
[0.27.0]: https://github.com/linebender/resvg/compare/v0.26.1...v0.27.0
[0.26.1]: https://github.com/linebender/resvg/compare/v0.26.0...v0.26.1
[0.26.0]: https://github.com/linebender/resvg/compare/v0.25.0...v0.26.0
[0.25.0]: https://github.com/linebender/resvg/compare/v0.24.0...v0.25.0
[0.24.0]: https://github.com/linebender/resvg/compare/v0.23.0...v0.24.0
[0.23.0]: https://github.com/linebender/resvg/compare/v0.22.0...v0.23.0
[0.22.0]: https://github.com/linebender/resvg/compare/v0.21.0...v0.22.0
[0.21.0]: https://github.com/linebender/resvg/compare/v0.20.0...v0.21.0
[0.20.0]: https://github.com/linebender/resvg/compare/v0.19.0...v0.20.0
[0.19.0]: https://github.com/linebender/resvg/compare/v0.18.0...v0.19.0
[0.18.0]: https://github.com/linebender/resvg/compare/v0.17.0...v0.18.0
[0.17.0]: https://github.com/linebender/resvg/compare/v0.16.0...v0.17.0
[0.16.0]: https://github.com/linebender/resvg/compare/v0.15.0...v0.16.0
[0.15.0]: https://github.com/linebender/resvg/compare/v0.14.1...v0.15.0
[0.14.1]: https://github.com/linebender/resvg/compare/v0.14.0...v0.14.1
[0.14.0]: https://github.com/linebender/resvg/compare/v0.13.1...v0.14.0
[0.13.1]: https://github.com/linebender/resvg/compare/v0.13.0...v0.13.1
[0.13.0]: https://github.com/linebender/resvg/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/linebender/resvg/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/linebender/resvg/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/linebender/resvg/compare/v0.9.1...v0.10.0
[0.9.1]: https://github.com/linebender/resvg/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/linebender/resvg/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/linebender/resvg/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/linebender/resvg/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/linebender/resvg/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/linebender/resvg/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/linebender/resvg/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/linebender/resvg/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/linebender/resvg/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/linebender/resvg/compare/v0.1.0...v0.2.0
