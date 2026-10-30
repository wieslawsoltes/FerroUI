//! Independent downstream compilation boundary for the settings library.
// These allowances are confined to the published emitter's generated code.
#[allow(clippy::if_same_then_else, clippy::clone_on_copy)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_xaml};
#[cfg(test)]
mod settings_tests;
