//! Text shaping with HarfBuzz.
//!
//! [`HarfBuzzTextShaper`] implements the text shaper contract of
//! `ferroui_base::platform`; [`HarfBuzzPlatform::initialize`] registers it.

mod harf_buzz_application_extensions;
mod harf_buzz_text_shaper;
#[cfg(test)]
mod harf_buzz_text_shaper_tests;
mod harf_buzz_typeface;
mod hb;

pub use harf_buzz_application_extensions::{HarfBuzzApplicationExtensions, HarfBuzzPlatform};
pub use harf_buzz_text_shaper::HarfBuzzTextShaper;
pub use harf_buzz_typeface::HarfBuzzTypeface;
