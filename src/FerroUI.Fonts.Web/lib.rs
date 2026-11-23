//! Web fonts for FerroUI: faces declared as with `@font-face`, loaded with the API of the CSS
//! Font Loading module, from WOFF and WOFF2 files, on every platform and rendering backend.
//!
//! An application opts in by depending on this crate, and nothing in the framework depends on
//! it. `docs/porting/web-fonts.md` is the plan it belongs to.

#![forbid(unsafe_code)]

mod brotli;
#[cfg(all(feature = "browser", target_family = "wasm"))]
mod browser_fetcher;
mod feature_shaper;
mod fetchers;
mod font_data_stream;
#[cfg(any(not(target_family = "wasm"), target_feature = "atomics"))]
mod off_thread;
mod text_invalidation;
mod web_font_collection;

pub use brotli::brotli_decoder_factory;
pub use ferroui_fonts_web_core as core;
pub use fetchers::{DefaultFontFaceFetcher, FontFetchOptions};
pub use font_data_stream::FontDataStream;
pub use web_font_collection::{FontFetchError, FontFetchResult, IFontFaceFetcher, WebFontCollection};
