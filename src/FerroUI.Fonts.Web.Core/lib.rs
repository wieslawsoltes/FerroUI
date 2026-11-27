//! The standards behind web fonts, as plain data in and plain data out.
//!
//! This crate has no dependency on the framework, does no I/O and starts no thread, so its
//! work can run on any thread. `docs/porting/web-fonts.md` is the plan it belongs to.
//!
//! - [`css`]: the `@font-face` rule and each of its descriptors (CSS Fonts 4 and 5).
//! - [`sfnt`]: the OpenType container and the sanitizer every font from the network goes through.

#![forbid(unsafe_code)]

pub mod css;
mod error;
mod limits;
pub mod sfnt;

pub use error::FontDecodeError;
pub use limits::WebFontLimits;
