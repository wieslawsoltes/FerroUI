//! The standards behind web fonts, as plain data in and plain data out.
//!
//! This crate has no dependency on the framework, does no I/O and starts no thread, so its
//! work can run on any thread. `docs/porting/web-fonts.md` is the plan it belongs to.
//!
//! - [`css`]: the `@font-face` rule and each of its descriptors (CSS Fonts 4 and 5).
//! - [`sfnt`]: the OpenType container and the sanitizer every font from the network goes through.
//! - [`loading`]: the states of the CSS Font Loading API and the `font-display` timeline.
//! - [`matching`]: which declared face a text uses (CSS Fonts 4, section 5.2).
//! - [`variations`]: what a variable font is set to for a request.
//! - [`FontDecoder`]: from the bytes of a file, whatever its container, to a sanitized font.

#![forbid(unsafe_code)]

pub mod css;
mod decoder;
mod error;
mod limits;
pub mod loading;
pub mod matching;
pub mod sfnt;
pub mod variations;
mod woff;
pub mod woff2;

pub use decoder::{decode_font, decode_font_with_brotli, sniff_container, FontContainer, FontDecoder};
pub use error::FontDecodeError;
pub use limits::WebFontLimits;
