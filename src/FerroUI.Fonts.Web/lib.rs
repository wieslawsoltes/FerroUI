//! Web fonts for FerroUI: faces declared as with `@font-face`, loaded with the API of the CSS
//! Font Loading module, from WOFF and WOFF2 files, on every platform and rendering backend.
//!
//! An application opts in by depending on this crate, and nothing in the framework depends on
//! it. `docs/porting/web-fonts.md` is the plan it belongs to.

#![forbid(unsafe_code)]

mod app_builder_extension;
mod brotli;
#[cfg(all(feature = "browser", target_family = "wasm"))]
mod browser_fetcher;
mod feature_shaper;
mod fetchers;
mod font_data_stream;
mod font_face_builder;
mod markup_types;
#[cfg(any(not(target_family = "wasm"), target_feature = "atomics"))]
mod off_thread;
mod rust_paths;
mod text_invalidation;
mod web_font_collection;
mod web_fonts;

#[cfg(test)]
mod xaml_tests;

pub use app_builder_extension::AppBuilderExtension;
pub use brotli::brotli_decoder_factory;
pub use ferroui_fonts_web_core as core;
pub use fetchers::{DefaultFontFaceFetcher, FontFetchOptions};
pub use font_data_stream::FontDataStream;
pub use font_face_builder::FontFaceBuilder;
pub use markup_types::{register_types, ASSEMBLY};
pub use web_font_collection::{FontFetchError, FontFetchResult, IFontFaceFetcher, WebFontCollection};
pub use web_fonts::WebFonts;
