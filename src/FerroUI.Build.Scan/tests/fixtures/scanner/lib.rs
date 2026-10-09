//! The fixture crate of the tests of the source scanner (`ferroui-build`, `scanner`).
//! It is read as files and never compiled: most of what it names does not exist.

pub mod controls;
mod macros;
mod markup_types;
pub mod media;
#[path = "odd/placed_elsewhere.rs"]
mod placed;
mod register_types;
mod rust_paths;
#[cfg(test)]
mod not_read;

pub use controls::decorator::Decorator;
pub use controls::Border;
pub use media::*;
pub use placed::Placed;
