//! The `@font-face` rule of CSS Fonts Level 4 and each of its descriptors.
//!
//! Each descriptor has its own parser, used both for a rule of a style sheet and for the
//! attributes of a `FontFace`. Input is untrusted: a parser returns an error and never panics.

mod descriptors;
mod palette;
mod source;
mod stylesheet;
mod tokenizer;
mod unicode_range;

#[cfg(test)]
mod tests;

pub use descriptors::{
    parse_family_name, parse_feature_settings, parse_font_display, parse_font_style, parse_font_weight,
    parse_font_width, parse_language_override, parse_metric_override, parse_named_instance, parse_size_adjust,
    parse_variation_settings, FontDisplay, FontFeatureSetting, FontStyleDescriptor, FontVariationSetting,
    FontWeightRange, FontWidthRange, MetricOverride,
};
pub use palette::{parse_base_palette, parse_override_colors, BasePalette, FontPaletteValues, PaletteColor};
pub use source::{parse_src, FontFormat, FontSource, FontTech, FontTechs};
pub use stylesheet::{parse_font_face_block, parse_font_palettes, parse_stylesheet, FontFaceDescriptors};
pub use unicode_range::{parse_unicode_range, UnicodeRangeSet};

use std::fmt;

/// Why a descriptor or a rule was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CssParseError {
    /// The value does not match the grammar of the descriptor.
    InvalidValue(&'static str),
    /// The rule has no valid `font-family` descriptor.
    MissingFamily,
    /// The rule has no valid `src` descriptor.
    MissingSource,
}

impl fmt::Display for CssParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue(descriptor) => write!(f, "invalid value for the descriptor {descriptor}"),
            Self::MissingFamily => f.write_str("the @font-face rule has no valid font-family"),
            Self::MissingSource => f.write_str("the @font-face rule has no valid src"),
        }
    }
}

impl std::error::Error for CssParseError {}
