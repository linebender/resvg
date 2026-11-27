// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::sync::Arc;

use crate::{Cache, Font, Text};

pub(crate) mod cache;
pub(crate) mod flatten;

mod colr;
#[cfg(feature = "fontdb")]
mod fontdb;
/// Provides access to the layout of a text node.
pub mod layout;

#[cfg(feature = "fontdb")]
pub use self::fontdb::FontdbResolver;

/// The ID of a glyph within a font
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u32);

impl From<GlyphId> for skrifa::raw::types::GlyphId {
    fn from(value: GlyphId) -> Self {
        Self::new(value.0)
    }
}

/// Identifies a font face within a [`FontResolver`].
///
/// Keys are chosen by the resolver. A key must refer to the same face for as
/// long as the resolver is used.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontKey(u64);

impl FontKey {
    /// Creates a key.
    pub const fn new(key: u64) -> Self {
        Self(key)
    }

    /// Returns the value of the key.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The data of a font face.
#[derive(Clone)]
pub struct FontData {
    /// The font file.
    pub data: Arc<dyn AsRef<[u8]> + Send + Sync>,
    /// The index of the face within a font collection, or 0.
    pub index: u32,
}

impl FontData {
    /// Creates font data from a font file and the index of a face in it.
    pub fn new(data: impl AsRef<[u8]> + Send + Sync + 'static, index: u32) -> Self {
        Self {
            data: Arc::new(data),
            index,
        }
    }

    /// Returns the font file.
    pub fn as_bytes(&self) -> &[u8] {
        (*self.data).as_ref()
    }
}

impl std::fmt::Debug for FontData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontData")
            .field("len", &self.as_bytes().len())
            .field("index", &self.index)
            .finish()
    }
}

/// Selects and loads the fonts used by `<text>` elements. Fonts are only
/// requested when text needs them.
pub trait FontResolver: Send + Sync {
    /// Selects the face to use for a font specification.
    fn select_font(&self, font: &Font) -> Option<FontKey>;

    /// Selects a face to fall back to for `c`.
    ///
    /// `base` is the face selected for `font`, and `used` lists the faces
    /// already tried, starting with `base`.
    fn select_fallback(
        &self,
        c: char,
        font: &Font,
        base: FontKey,
        used: &[FontKey],
    ) -> Option<FontKey>;

    /// Returns the data of a face returned by one of the selection methods.
    fn load(&self, key: FontKey) -> Option<FontData>;
}

impl<T: FontResolver + ?Sized> FontResolver for &T {
    fn select_font(&self, font: &Font) -> Option<FontKey> {
        (**self).select_font(font)
    }

    fn select_fallback(
        &self,
        c: char,
        font: &Font,
        base: FontKey,
        used: &[FontKey],
    ) -> Option<FontKey> {
        (**self).select_fallback(c, font, base, used)
    }

    fn load(&self, key: FontKey) -> Option<FontData> {
        (**self).load(key)
    }
}

impl std::fmt::Debug for dyn FontResolver + '_ {
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
pub(crate) fn convert(
    text: &mut Text,
    resolver: &dyn FontResolver,
    cache: &mut Cache,
) -> Option<()> {
    let (text_fragments, bbox) = layout::layout_text(text, resolver, &mut cache.fonts)?;
    for glyph in text_fragments
        .iter()
        .flat_map(|span| &span.positioned_glyphs)
    {
        cache.fonts.mark_used(glyph.font);
    }
    text.layouted = text_fragments;
    text.bounding_box = bbox.to_rect();
    text.abs_bounding_box = bbox.transform(text.abs_transform)?.to_rect();

    let (group, stroke_bbox) = flatten::flatten(text, &mut cache.fonts)?;
    text.flattened = Box::new(group);
    text.stroke_bounding_box = stroke_bbox.to_rect();
    text.abs_stroke_bounding_box = stroke_bbox.transform(text.abs_transform)?.to_rect();

    Some(())
}
