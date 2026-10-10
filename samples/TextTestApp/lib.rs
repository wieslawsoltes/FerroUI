//! text-test-app
//!
//! Port of the sample application `TextTestApp`: an interactive test of the text formatting
//! interface. A control formats the text of a text box as one line with the chosen font,
//! font features and size, and draws the line with its metrics, the bounds of its text and
//! runs, its caret stops and the distances of its characters; two lists show the shaped
//! buffers of the line (one row per glyph) and its character hits (one row per character),
//! and what is selected in them is drawn over the line. The entry point is `program.rs`.
//!
//! The sample is markup: `App.xaml` and `MainWindow.xaml`, each of which populates the class it
//! names. The documents are converted from the upstream sample by
//! `scripts/convert_catalog_xaml.py` (names only) and compiled by the build of the crate
//! (`build.rs`, docs/porting/xaml.md 9.5.22): a class is populated by the compiled markup of its
//! document.
//!
//! The files mirror the upstream sample: `InteractiveLineControl.cs` is
//! `interactive_line_control.rs`.

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as text_test_app;

mod app;
mod font_feature_collection_converter;
mod grid_row;
mod interactive_line_control;
mod main_window;
pub mod markup;
mod register_types;
mod selection_adorner;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use font_feature_collection_converter::FontFeatureCollectionConverter;
pub use grid_row::GridRow;
pub use interactive_line_control::InteractiveLineControl;
pub use main_window::{MainWindow, TextRunTag};
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};
pub use selection_adorner::SelectionAdorner;

#[cfg(test)]
mod tests;
