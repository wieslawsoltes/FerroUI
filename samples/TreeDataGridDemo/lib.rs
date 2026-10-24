//! treedatagrid-demo
//!
//! Port of the reference demo `TreeDataGridDemo` of the TreeDataGrid control: a window of tabs,
//! each a page that shows a TreeDataGrid over its own source. The entry point is `program.rs`.
//!
//! The pages of the reference live in the tabs of its main window; here every page is a view
//! of its own (`views`), which the main window shows in its tabs and the ControlCatalog in its
//! pages (docs/porting/treedatagrid.md, stage T7).
//!
//! The demo is markup: one document per view, which populates the class the document names,
//! compiled by the build of the crate (`build.rs`, docs/porting/xaml.md 9.5.22).

// The compiled markup of a document names the types of this crate by the name of the crate.
extern crate self as treedatagrid_demo;

#[cfg(feature = "app")]
mod app;
mod internal_tree_data_grid_source_extensions;
#[cfg(feature = "app")]
mod main_window;
pub mod markup;
mod register_types;

#[path = "Models/mod.rs"]
pub mod models;
#[path = "ViewModels/mod.rs"]
pub mod view_models;
#[path = "Views/mod.rs"]
pub mod views;

// One module per compiled document and `compiled_markup` (the loader table of the compiled
// markup of the crate and `register()`), which the build script of the crate generates.
ferroui_markup_xaml::include_compiled_xaml!();

#[cfg(feature = "app")]
pub use app::App;
#[cfg(feature = "app")]
pub use main_window::MainWindow;
pub use markup::SAMPLE;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(all(test, feature = "app"))]
mod tests;
