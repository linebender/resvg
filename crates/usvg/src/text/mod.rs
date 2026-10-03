// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::Arc;

use fontdb::{Database, ID};
use svgtypes::FontFamily;

use self::layout::DatabaseExt;
use crate::tree::BBox;
use crate::{Cache, Font, FontStretch, FontStyle, Group, LineJoin, Node, Text};

pub(crate) mod flatten;
mod transform;

mod colr;
/// Provides access to the layout of a text node.
pub mod layout;

/// The optical sizing variation axis tag.
pub(crate) const OPSZ: skrifa::Tag = skrifa::Tag::from_be_bytes(*b"opsz");

/// The ID of a glyph within a font
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u32);

impl From<GlyphId> for skrifa::raw::types::GlyphId {
    fn from(value: GlyphId) -> Self {
        Self::new(value.0)
    }
}

/// A shorthand for [FontResolver]'s font selection function.
///
/// This function receives a font specification (families + a style, weight,
/// stretch triple) and a font database and should return the ID of the font
/// that shall be used (if any).
///
/// In the basic case, the function will search the existing fonts in the
/// database to find a good match, e.g. via
/// [`Database::query`](fontdb::Database::query). This is what the [default
/// implementation](FontResolver::default_font_selector) does.
///
/// Users with more complex requirements can mutate the database to load
/// additional fonts dynamically. To perform mutation, it is recommended to call
/// `Arc::make_mut` on the provided database. (This call is not done outside of
/// the callback to not needless clone an underlying shared database if no
/// mutation will be performed.) It is important that the database is only
/// mutated additively. Removing fonts or replacing the entire database will
/// break things.
pub type FontSelectionFn<'a> =
    Box<dyn Fn(&Font, &mut Arc<Database>) -> Option<ID> + Send + Sync + 'a>;

/// A shorthand for [FontResolver]'s fallback selection function.
///
/// This function receives a specific character, a list of already used fonts,
/// and a font database. It should return the ID of a font that
/// - is not any of the already used fonts
/// - is as close as possible to the first already used font (if any)
/// - supports the given character
///
/// The function can search the existing database, but can also load additional
/// fonts dynamically. See the documentation of [`FontSelectionFn`] for more
/// details.
pub type FallbackSelectionFn<'a> =
    Box<dyn Fn(char, &[ID], &mut Arc<Database>) -> Option<ID> + Send + Sync + 'a>;

/// A font resolver for `<text>` elements.
///
/// This type can be useful if you want to have an alternative font handling to
/// the default one. By default, only fonts specified upfront in
/// [`Options::fontdb`](crate::Options::fontdb) will be used. This type allows
/// you to load additional fonts on-demand and customize the font selection
/// process.
pub struct FontResolver<'a> {
    /// Resolver function that will be used when selecting a specific font
    /// for a generic [`Font`] specification.
    pub select_font: FontSelectionFn<'a>,

    /// Resolver function that will be used when selecting a fallback font for a
    /// character.
    pub select_fallback: FallbackSelectionFn<'a>,
}

impl Default for FontResolver<'_> {
    fn default() -> Self {
        FontResolver {
            select_font: FontResolver::default_font_selector(),
            select_fallback: FontResolver::default_fallback_selector(),
        }
    }
}

impl FontResolver<'_> {
    /// Creates a default font selection resolver.
    ///
    /// The default implementation forwards to
    /// [`query`](fontdb::Database::query) on the font database specified in the
    /// [`Options`](crate::Options).
    pub fn default_font_selector() -> FontSelectionFn<'static> {
        Box::new(move |font, fontdb| {
            let mut name_list = Vec::new();
            for family in &font.families {
                name_list.push(match family {
                    FontFamily::Serif => fontdb::Family::Serif,
                    FontFamily::SansSerif => fontdb::Family::SansSerif,
                    FontFamily::Cursive => fontdb::Family::Cursive,
                    FontFamily::Fantasy => fontdb::Family::Fantasy,
                    FontFamily::Monospace => fontdb::Family::Monospace,
                    FontFamily::Named(s) => fontdb::Family::Name(s),
                });
            }

            // Use the default font as fallback.
            name_list.push(fontdb::Family::Serif);

            let stretch = match font.stretch {
                FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
                FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
                FontStretch::Condensed => fontdb::Stretch::Condensed,
                FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
                FontStretch::Normal => fontdb::Stretch::Normal,
                FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
                FontStretch::Expanded => fontdb::Stretch::Expanded,
                FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
                FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
            };

            let style = match font.style {
                FontStyle::Normal => fontdb::Style::Normal,
                FontStyle::Italic => fontdb::Style::Italic,
                FontStyle::Oblique => fontdb::Style::Oblique,
            };

            let query = fontdb::Query {
                families: &name_list,
                weight: fontdb::Weight(font.weight),
                stretch,
                style,
            };

            let id = fontdb.query(&query);
            if id.is_none() {
                log::warn!(
                    "No match for '{}' font-family.",
                    font.families
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }

            id
        })
    }

    /// Creates a default font fallback selection resolver.
    ///
    /// The default implementation searches through the entire `fontdb`
    /// to find a font that has the correct style and supports the character.
    pub fn default_fallback_selector() -> FallbackSelectionFn<'static> {
        Box::new(|c, exclude_fonts, fontdb| {
            let base_font_id = exclude_fonts[0];

            // Iterate over fonts and check if any of them support the specified char.
            for face in fontdb.faces() {
                // Ignore fonts, that were used for shaping already.
                if exclude_fonts.contains(&face.id) {
                    continue;
                }

                // Check that the new face has the same style.
                let base_face = fontdb.face(base_font_id)?;
                if base_face.style != face.style
                    && base_face.weight != face.weight
                    && base_face.stretch != face.stretch
                {
                    continue;
                }

                if !fontdb.has_char(face.id, c) {
                    continue;
                }

                let base_family = base_face
                    .families
                    .iter()
                    .find(|f| f.1 == fontdb::Language::English_UnitedStates)
                    .unwrap_or(&base_face.families[0]);

                let new_family = face
                    .families
                    .iter()
                    .find(|f| f.1 == fontdb::Language::English_UnitedStates)
                    .unwrap_or(&base_face.families[0]);

                log::warn!("Fallback from {} to {}.", base_family.0, new_family.0);
                return Some(face.id);
            }

            None
        })
    }
}

impl std::fmt::Debug for FontResolver<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FontResolver { .. }")
    }
}

/// Converts a text node into glyphs and positions them according to the rules
/// specified in the SVG specification. While doing so, we also calculate the
/// text bbox (which is not based on the outlines of a glyph, but instead the
/// glyph metrics as well as decoration spans).
///
/// Note that this only performs the text *layout*. The conversion of the
/// positioned glyphs into outlines ("flattening") is performed lazily,
/// either on demand via [`Text::flattened`](crate::Text::flattened) or
/// upfront for the whole tree via
/// [`Tree::compute_flattened_text`](crate::Tree::compute_flattened_text).
pub(crate) fn convert(text: &mut Text, resolver: &FontResolver, cache: &mut Cache) -> Option<()> {
    let (text_fragments, bbox) = layout::layout_text(text, resolver, &mut cache.fontdb)?;
    text.layouted = text_fragments;
    text.bounding_box = bbox.to_rect();
    text.abs_bounding_box = bbox.transform(text.abs_transform)?.to_rect();

    // Take a snapshot of the font database so that lazy flattening has access
    // to the same fonts (including ones loaded on demand during layout).
    text.fontdb = cache.fontdb.clone();

    // The stroke bounding box has to be known before flattening, because
    // ancestor group bounding boxes are calculated during parsing.
    // We approximate it from the per-glyph ink bounding boxes stored in the
    // font (no outline extraction required), the layout bounding box and
    // the decoration paths.
    let stroke_bbox = calculate_stroke_bbox(text).unwrap_or(bbox);
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();

    Some(())
}

/// Calculates an approximate ink bounding box of a text node, including stroke.
///
/// The returned bbox is the union of the per-glyph ink bounding boxes stored
/// in the font (expanded by the stroke width when the span is stroked) and
/// the decoration paths' stroke bounding boxes. When the ink bounds of a
/// glyph are not available, the layout (metrics) bounding box is used as a
/// fallback. The result is guaranteed to be at least as large as the ink of
/// the glyph outlines, which is what layer allocation during rendering
/// requires.
fn calculate_stroke_bbox(text: &Text) -> Option<tiny_skia_path::NonZeroRect> {
    use self::flatten::DatabaseExt as _;

    type BoundsKey = (ID, GlyphId, Vec<crate::FontVariation>);

    let mut bbox = BBox::default();
    let mut bounds_cache: std::collections::HashMap<BoundsKey, Option<tiny_skia_path::Rect>> =
        std::collections::HashMap::new();

    for span in &text.layouted {
        // The maximum distance the stroke can extend beyond the path.
        // Miter joins can extend up to `stroke-miterlimit * stroke-width / 2`
        // beyond the joint point; other joins at most `stroke-width / 2`.
        let stroke_expansion = span.stroke.as_ref().map(|stroke| {
            let half_width = stroke.width.get() / 2.0;
            match stroke.linejoin {
                LineJoin::Miter | LineJoin::MiterClip => {
                    half_width * stroke.miterlimit.get().max(1.0)
                }
                LineJoin::Round | LineJoin::Bevel => half_width,
            }
        });

        for glyph in &span.positioned_glyphs {
            let bounds = *bounds_cache
                .entry((glyph.font, glyph.id, span.variations.clone()))
                .or_insert_with(|| text.fontdb.bounds(glyph.font, glyph.id, &span.variations));

            // Glyph ink bounds are in font units with a Y-up orientation,
            // just like glyph outlines, so the outline transform applies.
            // When the ink bounds are not available (e.g. for glyphs
            // without a `glyf`/`CFF` outline), fall back to the layout
            // (metrics) bounding box of the whole text.
            let rect = bounds
                .and_then(|bounds| bounds.transform(glyph.outline_transform()))
                .unwrap_or(text.bounding_box);

            let rect = match stroke_expansion {
                Some(delta) => rect.outset(delta, delta).unwrap_or(rect),
                None => rect,
            };
            bbox = bbox.expand(rect);
        }

        for path in [&span.overline, &span.underline, &span.line_through]
            .into_iter()
            .flatten()
        {
            bbox = bbox.expand(path.stroke_bounding_box());
        }
    }

    bbox.to_non_zero_rect()
}

/// Flattens all text nodes in a group, recursively, sharing a single glyph cache.
pub(crate) fn flatten_group(parent: &Group, cache: &mut flatten::FlattenCache) {
    for node in &parent.children {
        match node {
            Node::Text(text) => {
                text.flatten_with_cache(cache);
            }
            Node::Group(group) => flatten_group(group, cache),
            _ => {}
        }

        node.subroots(|subroot| flatten_group(subroot, cache));
    }
}
