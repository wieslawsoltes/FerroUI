//! Native ProTranslate localization: compiled XAML, generated catalogs, live
//! English/Polish/Arabic switching, bound numeric formatting and automatic direction.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as sample_protranslate_demo;

/// Catalogs generated deterministically by `ferroui-protranslate-build`.
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/protranslate.rs"));
}

mod app;
mod main_window;
mod main_window_view_model;
pub mod markup;
mod register_types;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
// The XAML emitter uses Clone uniformly, including for Copy property values.
#[allow(clippy::clone_on_copy)]
mod compiled_xaml {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use compiled_xaml::*;

pub use app::App;
pub use main_window::MainWindow;
pub use main_window_view_model::MainWindowViewModel;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
