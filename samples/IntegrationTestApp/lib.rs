//! integration-test-app
//!
//! Port of the sample application `IntegrationTestApp`: the application the user interface
//! automation tests of the upstream project drive through the accessibility interface of the
//! platform. Its main window lists one page per control or platform feature (buttons, check
//! boxes, menus, popups, windows, screens, drag and drop, native embedding, ...); every control
//! a test looks for has a name or an automation id, which are the ones of the upstream sample.
//! The entry point is `program.rs`.
//!
//! The sample is markup: one document per page, which populates the class the document names.
//! The documents are converted from the upstream sample by `scripts/convert_catalog_xaml.py`
//! (names only) and compiled by the build of the crate (`build.rs`, docs/porting/xaml.md
//! 9.5.22): a class is populated by the compiled markup of its document.
//!
//! The directories and files mirror the upstream sample: the document `Pages/ButtonPage.xaml`
//! and its class in `Pages/button_page.rs`.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as integration_test_app;

mod app;
mod delegate_command;
mod mac_os_integration;
mod main_window;
pub mod markup;
mod register_types;
mod show_window_test;
mod topmost_window_test;

// The class of `Pages/EmbeddingPage.xaml` is a class of the root namespace in the upstream
// sample (`IntegrationTestApp.EmbeddingPage`), unlike the classes of the other pages: its
// module is a module of the root.
#[path = "Pages/embedding_page.rs"]
mod embedding_page;

#[path = "Embedding/mod.rs"]
pub mod embedding;
#[path = "Models/mod.rs"]
pub mod models;
#[path = "Pages/mod.rs"]
pub mod pages;
#[path = "ViewModels/mod.rs"]
pub mod view_models;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use delegate_command::DelegateCommand;
pub use embedding_page::EmbeddingPage;
pub use mac_os_integration::MacOSIntegration;
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};
pub use show_window_test::{MeasureBorder, ShowWindowTest};
pub use topmost_window_test::TopmostWindowTest;

use std::cell::Cell;

thread_local! {
    static OVERLAY_POPUPS: Cell<bool> = const { Cell::new(false) };
}

/// `Program.OverlayPopups`: whether the application was started with `--overlayPopups`. The
/// entry point (`program.rs`) is another target of the crate, so the value the main window
/// reads is kept here.
pub fn overlay_popups() -> bool {
    OVERLAY_POPUPS.with(Cell::get)
}

/// The setter of `Program.OverlayPopups` (private in the managed original): called by the
/// entry point before the application is built.
pub fn set_overlay_popups(value: bool) {
    OVERLAY_POPUPS.with(|overlay_popups| overlay_popups.set(value));
}

#[cfg(test)]
mod tests;
