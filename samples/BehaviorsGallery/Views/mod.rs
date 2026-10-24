//! The views of the gallery (namespace `BehaviorsGallery.Views`): the window, its view and the
//! sample view several pages show; the windows of the pages are under `Windows/`.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

pub mod main_view;
pub mod main_window;
pub mod sample_view;
#[path = "Windows/mod.rs"]
pub mod windows;

pub use main_view::MainView;
pub use main_window::MainWindow;
pub use sample_view::SampleView;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[MainView::TYPE, MainWindow::TYPE, SampleView::TYPE];

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[
    &MainView::XAML_CLASS,
    &MainWindow::XAML_CLASS,
    &SampleView::XAML_CLASS,
];

/// The types of this namespace declared with `ferro_markup_type!`: none.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// What the untyped value conversions must know about the types of this namespace: nothing.
pub(crate) fn register_value_types() {}
