//! Explicit locale-name boundary for keyboard XAML properties.
use ferroboard_core::text::{TextCulture, UnsupportedCulture};
pub(crate) fn name(value: Option<TextCulture>) -> Option<String> {
    value.map(|culture| culture.name().to_owned())
}
pub(crate) fn parse(value: Option<String>) -> Result<Option<TextCulture>, UnsupportedCulture> {
    value.as_deref().map(TextCulture::get).transpose()
}
