// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::Arc;

use fontdb::{Database, ID};
use svgtypes::FontFamily;

use self::layout::DatabaseExt;
use crate::{Cache, Font, Text};

pub(crate) mod flatten;

mod colr;
/// Provides access to the layout of a text node.
pub mod layout;

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

/// A fallback font selection request.
#[derive(Clone, Copy, Debug)]
pub struct FallbackRequest<'a> {
    /// The character that needs a fallback font.
    pub character: char,
    /// The font specification of the current text span.
    ///
    /// This is provided as context for missing-glyph fallback selection. It
    /// does not imply that the resolver controls text segmentation or the full
    /// `font-family` cascade for the text run.
    pub font: &'a Font,
    /// Fonts that have already been used while shaping the current run.
    pub exclude_fonts: &'a [ID],
}

/// A shorthand for [FontResolver]'s fallback selection function.
///
/// This function receives a fallback request and a font database. It should
/// return the ID of a font that
/// - is not any of the already used fonts
/// - is as close as possible to the first already used font (if any)
/// - supports the given character
///
/// The resolver is only responsible for selecting a candidate font for a
/// missing glyph. It does not control text segmentation, shaping, or a full
/// browser-style `font-family` cascade for the whole text run.
///
/// The function can search the existing database, but can also load additional
/// fonts dynamically. See the documentation of [`FontSelectionFn`] for more
/// details.
pub type FallbackSelectionFn<'a> =
    Box<dyn Fn(FallbackRequest<'_>, &mut Arc<Database>) -> Option<ID> + Send + Sync + 'a>;

fn fontdb_family(family: &FontFamily) -> fontdb::Family<'_> {
    match family {
        FontFamily::Serif => fontdb::Family::Serif,
        FontFamily::SansSerif => fontdb::Family::SansSerif,
        FontFamily::Cursive => fontdb::Family::Cursive,
        FontFamily::Fantasy => fontdb::Family::Fantasy,
        FontFamily::Monospace => fontdb::Family::Monospace,
        FontFamily::Named(s) => fontdb::Family::Name(s),
    }
}

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
    /// missing character.
    ///
    /// This callback only selects fallback font candidates. It does not control
    /// how text is split into runs or how shaping results are merged back into
    /// the laid out text.
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
                name_list.push(fontdb_family(family));
            }

            // Use the default font as fallback.
            name_list.push(fontdb::Family::Serif);

            let query = fontdb::Query {
                families: &name_list,
                weight: fontdb::Weight(font.weight),
                stretch: font.stretch.into(),
                style: font.style.into(),
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
    /// The default implementation first prefers fonts from the declared
    /// `font-family` list and then searches through the entire `fontdb`
    /// to find a font that has the correct style and supports the character.
    /// This still operates as missing-glyph fallback, not as a full text-run
    /// segmentation strategy.
    pub fn default_fallback_selector() -> FallbackSelectionFn<'static> {
        Box::new(|request, fontdb| {
            let Some(&base_font_id) = request.exclude_fonts.first() else {
                return None;
            };

            for family in request.font.families() {
                let family = fontdb_family(family);
                let query = fontdb::Query {
                    families: &[family],
                    weight: fontdb::Weight(request.font.weight()),
                    stretch: request.font.stretch().into(),
                    style: request.font.style().into(),
                };

                if let Some(id) = fontdb.query(&query) {
                    if !request.exclude_fonts.contains(&id)
                        && fontdb.has_char(id, request.character)
                    {
                        return Some(id);
                    }
                }
            }

            let base_face = fontdb.face(base_font_id)?;

            // Iterate over fonts and check if any of them support the specified char.
            for face in fontdb.faces() {
                // Ignore fonts, that were used for shaping already.
                if request.exclude_fonts.contains(&face.id) {
                    continue;
                }

                // Check that the new face has the same style.
                if base_face.style != face.style
                    && base_face.weight != face.weight
                    && base_face.stretch != face.stretch
                {
                    continue;
                }

                if !fontdb.has_char(face.id, request.character) {
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

/// Convert a text into its paths. This is done in two steps:
/// 1. We convert the text into glyphs and position them according to the rules specified
///    in the SVG specification. While doing so, we also calculate the text bbox (which
///    is not based on the outlines of a glyph, but instead the glyph metrics as well
///    as decoration spans).
/// 2. We convert all of the positioned glyphs into outlines.
pub(crate) fn convert(text: &mut Text, resolver: &FontResolver, cache: &mut Cache) -> Option<()> {
    let (text_fragments, bbox) = layout::layout_text(text, resolver, &mut cache.fontdb)?;
    text.layouted = text_fragments;
    text.bounding_box = bbox.to_rect();
    text.abs_bounding_box = bbox.transform(text.abs_transform)?.to_rect();

    let (group, stroke_bbox) = flatten::flatten(text, cache)?;
    text.flattened = Box::new(group);
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();

    Some(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use once_cell::sync::Lazy;

    use super::*;

    static TEST_FONTDB: Lazy<Arc<Database>> = Lazy::new(|| {
        let mut fontdb = Database::new();
        let fonts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resvg/tests/fonts");
        fontdb.load_fonts_dir(fonts_dir);
        Arc::new(fontdb)
    });

    fn test_font(families: &[&str]) -> Font {
        Font {
            families: families
                .iter()
                .map(|family| FontFamily::Named((*family).to_string()))
                .collect(),
            style: crate::FontStyle::Normal,
            stretch: crate::FontStretch::Normal,
            weight: 400,
            variations: Vec::new(),
        }
    }

    #[test]
    fn default_fallback_selector_prefers_declared_families() {
        let mut fontdb = TEST_FONTDB.clone();
        let font = test_font(&["Noto Sans", "Noto Sans Devanagari"]);

        let select_font = FontResolver::default_font_selector();
        let base_font_id = select_font(&font, &mut fontdb).unwrap();

        let select_fallback = FontResolver::default_fallback_selector();
        let fallback_id = select_fallback(
            FallbackRequest {
                character: 'क',
                font: &font,
                exclude_fonts: &[base_font_id],
            },
            &mut fontdb,
        )
        .unwrap();

        let face = fontdb.face(fallback_id).unwrap();
        assert!(
            face.families
                .iter()
                .any(|family| family.0 == "Noto Sans Devanagari")
        );
    }
}
