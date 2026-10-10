//! app-without-lifetime
//!
//! Port of the sample application `AppWithoutLifetime`: an application that is run without an
//! application lifetime. The entry point (`program.rs`) sets the application up, creates the
//! main window itself and runs the main loop until that window is closed; the button of the
//! main window opens a second window owned by it.
//!
//! The sample is markup: `App.xaml`, `MainWindow.xaml` and `Sub.xaml`, each of which populates
//! the class it names. The documents are converted from the upstream sample by
//! `scripts/convert_catalog_xaml.py` (names only) and compiled by the build of the crate
//! (`build.rs`, docs/porting/xaml.md 9.5.22): a class is populated by the compiled markup of its
//! document.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as app_without_lifetime;

mod app;
mod main_window;
pub mod markup;
mod register_types;
mod sub;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};
pub use sub::Sub;

#[cfg(test)]
mod tests;
