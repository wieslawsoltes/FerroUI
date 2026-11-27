//! Declaring a face from code.

use crate::WebFontCollection;
use ferroui_fonts_web_core::css::{parse_src, CssParseError, FontFaceDescriptors};
use ferroui_fonts_web_core::loading::FontFaceId;

/// A face being declared, with the descriptors of `@font-face` in their CSS text:
///
/// ```ignore
/// fonts.declare("Inter").source("url(/fonts/Inter.woff2) format(woff2)").weight("100 900").add()?;
/// ```
#[must_use = "a face is declared by `add`"]
pub struct FontFaceBuilder<'c> {
    collection: &'c WebFontCollection,
    family: String,
    source: Option<String>,
    descriptors: Vec<(&'static str, String)>,
}

impl<'c> FontFaceBuilder<'c> {
    pub(crate) fn new(collection: &'c WebFontCollection, family: &str) -> Self {
        Self { collection, family: family.to_owned(), source: None, descriptors: Vec::new() }
    }

    /// The value of `src`.
    pub fn source(mut self, value: &str) -> Self {
        self.source = Some(value.to_owned());
        self
    }

    fn descriptor(mut self, name: &'static str, value: &str) -> Self {
        self.descriptors.push((name, value.to_owned()));
        self
    }

    pub fn style(self, value: &str) -> Self {
        self.descriptor("font-style", value)
    }

    pub fn weight(self, value: &str) -> Self {
        self.descriptor("font-weight", value)
    }

    pub fn stretch(self, value: &str) -> Self {
        self.descriptor("font-width", value)
    }

    pub fn unicode_range(self, value: &str) -> Self {
        self.descriptor("unicode-range", value)
    }

    pub fn display(self, value: &str) -> Self {
        self.descriptor("font-display", value)
    }

    pub fn feature_settings(self, value: &str) -> Self {
        self.descriptor("font-feature-settings", value)
    }

    pub fn variation_settings(self, value: &str) -> Self {
        self.descriptor("font-variation-settings", value)
    }

    pub fn named_instance(self, value: &str) -> Self {
        self.descriptor("font-named-instance", value)
    }

    pub fn language_override(self, value: &str) -> Self {
        self.descriptor("font-language-override", value)
    }

    pub fn ascent_override(self, value: &str) -> Self {
        self.descriptor("ascent-override", value)
    }

    pub fn descent_override(self, value: &str) -> Self {
        self.descriptor("descent-override", value)
    }

    pub fn line_gap_override(self, value: &str) -> Self {
        self.descriptor("line-gap-override", value)
    }

    pub fn size_adjust(self, value: &str) -> Self {
        self.descriptor("size-adjust", value)
    }

    /// Declares the face. A value that is not valid for its descriptor is an error here, where
    /// a style sheet would ignore it.
    pub fn add(self) -> Result<FontFaceId, CssParseError> {
        if self.family.trim().is_empty() {
            return Err(CssParseError::MissingFamily);
        }
        let source = self.source.ok_or(CssParseError::MissingSource)?;
        let sources = parse_src(&source).map_err(|_| CssParseError::MissingSource)?;
        let mut face = FontFaceDescriptors::new(self.family.trim(), sources);
        for (name, value) in &self.descriptors {
            face.set_descriptor(name, value)?;
        }
        self.collection.add(face).map_err(|_| CssParseError::InvalidValue("font face"))
    }
}
