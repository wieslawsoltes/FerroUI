//! control-catalog
//!
//! Port of the sample application `ControlCatalog` (the shared library:
//! pages, view models, models, converters and assets), the validation
//! application of the port. The entry point is the crate
//! `control-catalog-desktop`.
//!
//! The sample is markup: one document per page, which populates the class
//! the document names. The documents are converted from the upstream sample
//! by `scripts/sync-control-catalog.sh` (names only) and compiled by the
//! build of the crate (`build.rs`, docs/porting/xaml.md 9.5.22): a class is
//! populated by the compiled markup of its document, and a document of the
//! assembly `ControlCatalog` is addressable as
//! `ferres://ControlCatalog/<path>` through the loader table of the compiled
//! markup. Documents that do not load yet are listed in `excluded.txt` with
//! what they wait for. With the feature `runtime-markup` the documents are
//! embedded as assets instead and loaded by the run-time loader.
//!
//! The directories and files mirror the upstream sample: the document
//! `Pages/ButtonsPage.xaml` and its class in `Pages/buttons_page.rs`.

// The compiled markup of a document without a class (`CustomThemes.xaml`) names the types of
// this crate by the name of the crate, as the compiled markup of another crate would; the
// compiled markup of a class names them through `crate`.
extern crate self as control_catalog;

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod app;
mod assets;
mod decorated_window;
mod icons;
mod main_view;
mod main_window;
pub mod markup;
mod page_assets;
mod register_types;
mod smoke;
mod transparent_styles;

#[path = "Controls/mod.rs"]
pub mod controls;
#[path = "Converter/mod.rs"]
pub mod converter;
#[path = "Models/mod.rs"]
pub mod models;
#[path = "Pages/mod.rs"]
pub mod pages;
#[path = "ViewModels/mod.rs"]
pub mod view_models;
#[path = "Views/mod.rs"]
pub mod views;

// One module per compiled document (`compiled_pages_border_page`, ..: the function `populate`
// of the document of a class, the build function of a document without one) and
// `compiled_markup` (the loader table of the compiled markup of the crate and `register()`),
// which the build script of the crate generates.
#[cfg(not(feature = "runtime-markup"))]
ferroui_markup_xaml::include_compiled_xaml!();

pub use app::App;
pub use assets::{documents, excluded, excluded_documents, ExcludedDocument};
pub use decorated_window::DecoratedWindow;
pub use main_view::MainView;
pub use main_window::MainWindow;
pub use page_assets::{IPageAssets, PageAssets, PageAssetsFuture};
pub use register_types::{register_types, ASSEMBLY};
pub use smoke::{show_every_page, show_pages};
pub use transparent_styles::TransparentStyles;

/// The classes of the root namespace `ControlCatalog` (`X::TYPE`).
pub(crate) const ROOT_TYPES: &[&TypeInfo] = &[App::TYPE, DecoratedWindow::TYPE, MainView::TYPE, MainWindow::TYPE, TransparentStyles::TYPE];

/// The classes of the root namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const ROOT_CLASSES: &[&XamlClass] =
    &[&App::XAML_CLASS, &DecoratedWindow::XAML_CLASS, &MainView::XAML_CLASS, &MainWindow::XAML_CLASS, &TransparentStyles::XAML_CLASS];

/// The types of the root namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`.
pub(crate) const ROOT_MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of the root namespace.
pub(crate) fn register_root_value_types() {}

#[cfg(test)]
mod tests;
