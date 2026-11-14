// Copyright 2024 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use fontdb::{Database, ID};
use svgtypes::FontFamily;

use super::{FontData, FontKey, FontResolver};
use crate::{Font, FontStretch, FontStyle};

/// A [`FontResolver`] that selects fonts from a [`fontdb::Database`].
pub struct FontdbResolver {
    state: LazyLock<FontdbState, Box<dyn FnOnce() -> FontdbState + Send>>,
}

struct FontdbState {
    db: Arc<Database>,
    ids: Vec<ID>,
    keys: HashMap<ID, FontKey>,
}

impl FontdbState {
    fn new(db: Arc<Database>) -> Self {
        let ids: Vec<ID> = db.faces().map(|face| face.id).collect();
        let keys = ids
            .iter()
            .enumerate()
            .map(|(i, id)| (*id, FontKey::new(i as u64)))
            .collect();
        Self { db, ids, keys }
    }

    fn id(&self, key: FontKey) -> Option<ID> {
        self.ids.get(usize::try_from(key.get()).ok()?).copied()
    }
}

impl FontdbResolver {
    /// Creates a resolver whose database is created by `load` when text first
    /// needs a font.
    ///
    /// If `load` panics, every later use of the resolver panics as well.
    pub fn new<D: Into<Arc<Database>>>(load: impl FnOnce() -> D + Send + 'static) -> Self {
        Self {
            state: LazyLock::new(Box::new(move || FontdbState::new(load().into()))),
        }
    }

    /// Creates a resolver that loads the system fonts when text first needs a
    /// font.
    #[cfg(feature = "system-fonts")]
    pub fn system() -> Self {
        Self::new(|| {
            let mut db = Database::new();
            db.load_system_fonts();
            db
        })
    }
}

impl Default for FontdbResolver {
    fn default() -> Self {
        Self::new(Database::new)
    }
}

impl std::fmt::Debug for FontdbResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FontdbResolver { .. }")
    }
}

impl FontResolver for FontdbResolver {
    fn select_font(&self, font: &Font) -> Option<FontKey> {
        let state = &*self.state;

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

        let key = state
            .db
            .query(&query)
            .and_then(|id| state.keys.get(&id).copied());
        if key.is_none() {
            log::warn!(
                "No match for '{}' font-family.",
                font.families
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        key
    }

    fn select_fallback(
        &self,
        c: char,
        _: &Font,
        base: FontKey,
        used: &[FontKey],
    ) -> Option<FontKey> {
        let state = &*self.state;
        let base_face = state.db.face(state.id(base)?)?;

        for (i, &id) in state.ids.iter().enumerate() {
            let key = FontKey::new(i as u64);
            if used.contains(&key) {
                continue;
            }

            let Some(face) = state.db.face(id) else {
                continue;
            };

            if base_face.style != face.style
                && base_face.weight != face.weight
                && base_face.stretch != face.stretch
            {
                continue;
            }

            if !has_char(&state.db, face.id, c) {
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
                .unwrap_or(&face.families[0]);

            log::warn!("Fallback from {} to {}.", base_family.0, new_family.0);
            return Some(key);
        }

        None
    }

    fn load(&self, key: FontKey) -> Option<FontData> {
        let state = &*self.state;
        let id = state.id(key)?;

        // Share the font data if it is already in memory, otherwise copy it.
        match state.db.face_source(id)? {
            (fontdb::Source::Binary(data), index) => Some(FontData { data, index }),
            #[cfg(feature = "memmap-fonts")]
            (fontdb::Source::SharedFile(_, data), index) => Some(FontData { data, index }),
            #[allow(unreachable_patterns)]
            _ => state.db.with_face_data(id, |data, index| FontData {
                data: Arc::new(data.to_vec()),
                index,
            }),
        }
    }
}

#[inline(never)]
fn has_char(fontdb: &Database, id: ID, c: char) -> bool {
    // Unlike for the main fonts where we use skrifa's owned font abstraction,
    // here we just call `with_face_data` repeatedly so we don't need to call `to_vec`
    // for all fallback fonts.
    // TODO: Cache the charmaps of fallback candidates, building one for every lookup is expensive.
    fontdb
        .with_face_data(id, |data, face_index| {
            let font = skrifa::FontRef::from_index(data, face_index).ok()?;
            skrifa::charmap::Charmap::new(&font).map(c)
        })
        .flatten()
        .is_some()
}
