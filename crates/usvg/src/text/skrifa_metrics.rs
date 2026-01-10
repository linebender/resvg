// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Font metrics extraction using skrifa.
//!
//! This module provides an alternative to ttf-parser for extracting font metrics,
//! using skrifa's MetadataProvider trait. This is used when the `hinting` feature
//! is enabled to reduce double font parsing.

use std::num::NonZeroU16;

use fontdb::ID;
use skrifa::{
    instance::LocationRef, instance::Size as SkrifaSize, raw::TableProvider, FontRef,
    MetadataProvider,
};

use super::layout::ResolvedFont;

/// Load font metrics using skrifa's MetadataProvider.
///
/// Returns a ResolvedFont containing all necessary metrics for text layout.
pub fn load_font_metrics(data: &[u8], face_index: u32, id: ID) -> Option<ResolvedFont> {
    let font = FontRef::from_index(data, face_index).ok()?;
    let metrics = font.metrics(SkrifaSize::unscaled(), LocationRef::default());

    let units_per_em = NonZeroU16::new(metrics.units_per_em)?;

    // skrifa provides ascent/descent as f32 in font units (when using unscaled size)
    let ascent = metrics.ascent as i16;
    let descent = metrics.descent as i16;

    // x_height is optional in skrifa
    let x_height = metrics
        .x_height
        .and_then(|x| u16::try_from(x as i32).ok())
        .and_then(NonZeroU16::new);
    let x_height = match x_height {
        Some(height) => height,
        None => {
            // If not set - fallback to height * 45%.
            // 45% is what Firefox uses.
            u16::try_from((f32::from(ascent - descent) * 0.45) as i32)
                .ok()
                .and_then(NonZeroU16::new)?
        }
    };

    // Get strikeout/line-through position from skrifa's strikeout decoration
    let line_through_position = match metrics.strikeout {
        Some(decoration) => decoration.offset as i16,
        None => x_height.get() as i16 / 2,
    };

    // Get underline metrics from skrifa
    let (underline_position, underline_thickness) = match metrics.underline {
        Some(decoration) => {
            let thickness = u16::try_from(decoration.thickness as i32)
                .ok()
                .and_then(NonZeroU16::new)
                // skrifa guarantees that units_per_em is >= 16
                .unwrap_or_else(|| NonZeroU16::new(units_per_em.get() / 12).unwrap());

            (decoration.offset as i16, thickness)
        }
        None => (
            -(units_per_em.get() as i16) / 9,
            NonZeroU16::new(units_per_em.get() / 12).unwrap(),
        ),
    };

    // Get subscript/superscript metrics from OS/2 table, fall back to calculation
    // 0.2 and 0.4 are generic offsets used by some applications (Inkscape/librsvg).
    let mut subscript_offset = (units_per_em.get() as f32 / 0.2).round() as i16;
    let mut superscript_offset = (units_per_em.get() as f32 / 0.4).round() as i16;

    // Try to get actual values from OS/2 table
    if let Ok(os2) = font.os2() {
        subscript_offset = os2.y_subscript_y_offset();
        superscript_offset = os2.y_superscript_y_offset();
    }

    Some(ResolvedFont::new(
        id,
        units_per_em,
        ascent,
        descent,
        x_height,
        underline_position,
        underline_thickness,
        line_through_position,
        subscript_offset,
        superscript_offset,
    ))
}

/// Check if a font contains a glyph for the given character using skrifa's charmap.
pub fn has_char(data: &[u8], face_index: u32, c: char) -> bool {
    let font = match FontRef::from_index(data, face_index) {
        Ok(f) => f,
        Err(_) => return false,
    };

    font.charmap().map(c).is_some()
}
