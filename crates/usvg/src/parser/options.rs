// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(feature = "text")]
use std::sync::Arc;

#[cfg(feature = "text")]
use crate::{Font, FontData, FontKey, FontResolver};
use crate::{ImageHrefResolver, ImageRendering, ShapeRendering, Size, TextRendering};

#[cfg(feature = "text")]
struct NoFonts;

#[cfg(feature = "text")]
impl FontResolver for NoFonts {
    fn select_font(&self, _: &Font) -> Option<FontKey> {
        None
    }

    fn select_fallback(&self, _: char, _: &Font, _: FontKey, _: &[FontKey]) -> Option<FontKey> {
        None
    }

    fn load(&self, _: FontKey) -> Option<FontData> {
        None
    }
}

#[cfg(feature = "text")]
static NO_FONTS: std::sync::LazyLock<Arc<NoFonts>> = std::sync::LazyLock::new(|| Arc::new(NoFonts));

/// The default nesting limit when parsing.
pub const DEFAULT_MAX_DEPTH: u32 = 1024;

/// Processing options.
#[derive(Debug)]
pub struct Options<'a> {
    /// Directory that will be used during relative paths resolving.
    ///
    /// Expected to be the same as the directory that contains the SVG file,
    /// but can be set to any.
    ///
    /// Default: `None`
    pub resources_dir: Option<std::path::PathBuf>,

    /// Target DPI.
    ///
    /// Impacts units conversion.
    ///
    /// Default: 96.0
    pub dpi: f32,

    /// A default font family.
    ///
    /// Will be used when no `font-family` attribute is set in the SVG.
    ///
    /// Default: Times New Roman
    pub font_family: String,

    /// A default font size.
    ///
    /// Will be used when no `font-size` attribute is set in the SVG.
    ///
    /// Default: 12
    pub font_size: f32,

    /// A list of languages.
    ///
    /// Will be used to resolve a `systemLanguage` conditional attribute.
    ///
    /// Format: en, en-US.
    ///
    /// Default: `[en]`
    pub languages: Vec<String>,

    /// Specifies the default shape rendering method.
    ///
    /// Will be used when an SVG element's `shape-rendering` property is set to `auto`.
    ///
    /// Default: GeometricPrecision
    pub shape_rendering: ShapeRendering,

    /// Specifies the default text rendering method.
    ///
    /// Will be used when an SVG element's `text-rendering` property is set to `auto`.
    ///
    /// Default: OptimizeLegibility
    pub text_rendering: TextRendering,

    /// Specifies the default image rendering method.
    ///
    /// Will be used when an SVG element's `image-rendering` property is set to `auto`.
    ///
    /// Default: OptimizeQuality
    pub image_rendering: ImageRendering,

    /// Default viewport size to assume if there is no `viewBox` attribute and
    /// the `width` or `height` attributes are relative.
    ///
    /// Default: `(100, 100)`
    pub default_size: Size,

    /// Specifies the way `xlink:href` in `<image>` elements should be handled.
    ///
    /// Default: see type's documentation for details
    pub image_href_resolver: ImageHrefResolver<'a>,

    /// Selects and loads the fonts used by text.
    ///
    /// Default: a resolver without any fonts.
    #[cfg(feature = "text")]
    pub font_resolver: Arc<dyn FontResolver + 'a>,
    /// A CSS stylesheet that should be injected into the SVG. Can be used to overwrite
    /// certain attributes.
    pub style_sheet: Option<String>,

    /// The maximum nesting allowed while parsing. If this limit is exceeded,
    /// the parser will return an error.
    pub max_depth: u32,
}

impl Default for Options<'_> {
    fn default() -> Options<'static> {
        Options {
            resources_dir: None,
            dpi: 96.0,
            // Default font is user-agent dependent so we can use whichever we like.
            font_family: "Times New Roman".to_owned(),
            font_size: 12.0,
            languages: vec!["en".to_string()],
            shape_rendering: ShapeRendering::default(),
            text_rendering: TextRendering::default(),
            image_rendering: ImageRendering::default(),
            default_size: Size::from_wh(100.0, 100.0).unwrap(),
            image_href_resolver: ImageHrefResolver::default(),
            #[cfg(feature = "text")]
            font_resolver: NO_FONTS.clone(),
            style_sheet: None,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

impl Options<'_> {
    /// Converts a relative path into absolute relative to the SVG file itself.
    ///
    /// If `Options::resources_dir` is not set, returns itself.
    pub fn get_abs_path(&self, rel_path: &std::path::Path) -> std::path::PathBuf {
        match self.resources_dir {
            Some(ref dir) => dir.join(rel_path),
            None => rel_path.into(),
        }
    }
}
