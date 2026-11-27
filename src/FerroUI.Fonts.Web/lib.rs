//! Web fonts for FerroUI: faces declared as with `@font-face`, loaded with the API of the CSS
//! Font Loading module, from WOFF and WOFF2 files, on every platform and rendering backend.
//!
//! An application opts in by depending on this crate, and nothing in the framework depends on
//! it. `docs/porting/web-fonts.md` is the plan it belongs to.

#![forbid(unsafe_code)]

mod brotli;

pub use brotli::brotli_decoder_factory;
pub use ferroui_fonts_web_core as core;
