//! Standalone downstream integration through public FerroBoard/FerroUI APIs.
extern crate self as ferroboard_native_consumer;
pub mod consumer;
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_xaml};
#[cfg(test)]
mod tests;
