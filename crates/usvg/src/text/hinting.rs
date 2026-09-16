// Copyright 2026 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Font hinting configuration.

/// The hinting engine to use.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum FontHintingEngine {
    /// The TrueType or PostScript interpreter, i.e. the hints embedded in the
    /// font itself.
    Interpreter,
    /// The automatic hinter, which adjusts outlines without relying on hints
    /// embedded in the font.
    Auto,
    /// Picks the interpreter for fonts that carry hints and the automatic
    /// hinter for those that don't. This is what FreeType does by default.
    #[default]
    AutoFallback,
}

/// The basic mode for [`FontHintingTarget::Smooth`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum FontHintingSmoothMode {
    /// The standard smooth hinting mode.
    #[default]
    Normal,
    /// Hinting with a lighter touch, meaning less aggressive adjustment in the
    /// horizontal direction.
    Light,
    /// Hinting optimized for subpixel rendering with horizontal LCD layouts.
    Lcd,
    /// Hinting optimized for subpixel rendering with vertical LCD layouts.
    VerticalLcd,
}

/// The rasterization the hinted outline is being prepared for.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FontHintingTarget {
    /// A strong hinting style intended for aliased, monochrome rasterization.
    ///
    /// Since resvg anti-aliases text, this mostly serves to align stems to the
    /// pixel grid as aggressively as possible. Note that the TrueType
    /// interpreter largely ignores the distinction between this and
    /// [`FontHintingTarget::Smooth`], so it mainly takes effect together with
    /// [`FontHintingEngine::Auto`].
    Mono,
    /// A hinting style suitable for anti-aliased rasterization.
    Smooth {
        /// The basic mode for smooth hinting.
        mode: FontHintingSmoothMode,
        /// If true, TrueType bytecode may assume that the outline will be
        /// rasterized with vertical supersampling.
        ///
        /// Disabling this makes ClearType fonts produce narrower horizontal
        /// stems, which suits an analytical area rasterizer such as the one in
        /// tiny-skia.
        symmetric_rendering: bool,
        /// If true, the hinting engine may not adjust the glyph advance.
        preserve_linear_metrics: bool,
    },
}

impl Default for FontHintingTarget {
    /// The same defaults skrifa uses for a smooth target.
    fn default() -> Self {
        Self::Smooth {
            mode: FontHintingSmoothMode::Normal,
            symmetric_rendering: true,
            preserve_linear_metrics: false,
        }
    }
}

/// Font hinting configuration.
///
/// Hinting grid-fits glyph outlines so that stems align to whole pixels, which
/// improves legibility of small text. It is disabled by default, because an SVG
/// is resolution independent while hinting is not: outlines have to be fitted
/// for a specific pixel grid, and usvg has to commit to one while flattening
/// text into paths, before the scale a `Tree` is eventually rendered at is
/// known.
///
/// The grid is derived from the font size in user units, so hinted output only
/// lands on whole pixels when the tree is rendered without scaling, or at an
/// integer zoom factor.
///
/// Hinting never changes glyph positions, only the outlines themselves. Text
/// therefore occupies the same space whether it is hinted or not.
///
/// Elements with `text-rendering="geometricPrecision"` are never hinted, since
/// that property asks for exact outlines.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct FontHintingOptions {
    /// The hinting engine to use.
    pub engine: FontHintingEngine,
    /// The rasterization the outline is being prepared for.
    pub target: FontHintingTarget,
}

impl From<FontHintingEngine> for skrifa::outline::Engine {
    fn from(engine: FontHintingEngine) -> Self {
        match engine {
            FontHintingEngine::Interpreter => Self::Interpreter,
            FontHintingEngine::Auto => Self::Auto(None),
            FontHintingEngine::AutoFallback => Self::AutoFallback,
        }
    }
}

impl From<FontHintingSmoothMode> for skrifa::outline::SmoothMode {
    fn from(mode: FontHintingSmoothMode) -> Self {
        match mode {
            FontHintingSmoothMode::Normal => Self::Normal,
            FontHintingSmoothMode::Light => Self::Light,
            FontHintingSmoothMode::Lcd => Self::Lcd,
            FontHintingSmoothMode::VerticalLcd => Self::VerticalLcd,
        }
    }
}

impl From<FontHintingTarget> for skrifa::outline::Target {
    fn from(target: FontHintingTarget) -> Self {
        match target {
            FontHintingTarget::Mono => Self::Mono,
            FontHintingTarget::Smooth {
                mode,
                symmetric_rendering,
                preserve_linear_metrics,
            } => Self::Smooth {
                mode: mode.into(),
                symmetric_rendering,
                preserve_linear_metrics,
            },
        }
    }
}

impl From<FontHintingOptions> for skrifa::outline::HintingOptions {
    fn from(options: FontHintingOptions) -> Self {
        Self {
            engine: options.engine.into(),
            target: options.target.into(),
        }
    }
}
