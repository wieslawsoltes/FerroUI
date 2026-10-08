//! control-catalog
//!
//! Port of the sample application `ControlCatalog` (the shared library:
//! pages, view models, models, converters and assets), the validation
//! application of the port. The entry point is the crate
//! `control-catalog-desktop`.
//!
//! The sample is markup: one document per page, loaded by the class the
//! document names. The documents are converted from the upstream sample by
//! `scripts/sync-control-catalog.sh` (names only) and embedded as assets of
//! the assembly `ControlCatalog`, addressable as
//! `ferres://ControlCatalog/<path>`. Documents that do not load yet are
//! listed in `excluded.txt` with what they wait for.
//!
//! The directories and files mirror the upstream sample: the document
//! `Pages/ButtonsPage.xaml` and its class in `Pages/buttons_page.rs`.

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

pub use app::App;
pub use assets::{documents, excluded, excluded_documents, ExcludedDocument};
pub use decorated_window::DecoratedWindow;
pub use main_view::MainView;
pub use main_window::MainWindow;
pub use page_assets::{IPageAssets, PageAssets, PageAssetsFuture};
pub use register_types::{register_types, ASSEMBLY};
pub use smoke::show_every_page;
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
