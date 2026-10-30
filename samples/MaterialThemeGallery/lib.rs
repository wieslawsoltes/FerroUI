//! Android Material 3 Expressive control gallery and deterministic capture harness.
// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as material_theme_gallery;

mod app;
pub mod capture;
mod capture_host;

mod main_window;
pub mod markup;
mod register_types;
pub mod theme;

#[path = "Pages/mod.rs"]
pub mod pages;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use capture_host::CaptureHost;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use pages::{PageInfo, PAGES};
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
