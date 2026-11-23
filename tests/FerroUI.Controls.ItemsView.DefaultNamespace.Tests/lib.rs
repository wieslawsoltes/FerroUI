//! ItemsView resolves from the default XML namespace, in compiled and in run-time loaded markup.
extern crate self as ferroui_controls_itemsview_default_namespace_tests;

// These allowances are confined to the generated code of the compiler.
#[allow(clippy::if_same_then_else, clippy::clone_on_copy)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_xaml};

#[cfg(test)]
mod tests;
