// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! SVG filter types.

use core::ops::Deref;

use crate::{Group, NonEmptyString, NonZeroRect};

/// A filter element.
///
/// `filter` element in the SVG. It dereferences to its region and primitives in user space.
#[derive(Debug)]
pub struct Filter {
    pub(crate) id: NonEmptyString,
    pub(crate) filter: filtra_types::Filter<Group>,
}

impl Filter {
    /// Element's ID.
    ///
    /// Taken from the SVG itself.
    /// Used only during SVG writing. `resvg` doesn't rely on this property.
    pub fn id(&self) -> &str {
        self.id.get()
    }

    /// Filter region.
    ///
    /// `x`, `y`, `width` and `height` in the SVG.
    pub fn rect(&self) -> NonZeroRect {
        let region = self.filter.region;
        NonZeroRect::from_xywh(region.x, region.y, region.width, region.height).unwrap()
    }
}

impl Deref for Filter {
    type Target = filtra_types::Filter<Group>;

    fn deref(&self) -> &Self::Target {
        &self.filter
    }
}
