//! sandbox
//!
//! Port of the sample application `Sandbox`: an application under the Fluent theme whose main
//! window is empty, the place to try a piece of markup. The entry point is `program.rs`.
//!
//! The sample is markup: `App.xaml` and `MainWindow.xaml`, each of which populates the class it
//! names. The documents are converted from the upstream sample by
//! `scripts/convert_catalog_xaml.py` (names only) and compiled by the build of the crate
//! (`build.rs`, docs/porting/xaml.md 9.5.22): a class is populated by the compiled markup of its
//! document.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as sandbox;

mod app;
mod main_window;
pub mod markup;
mod register_types;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
