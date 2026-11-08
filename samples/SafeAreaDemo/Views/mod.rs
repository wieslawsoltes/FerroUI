//! The views of the sample (namespace `SafeAreaDemo.Views`): one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::TypeInfo;

mod main_view;
mod main_window;

pub use main_view::MainView;
pub use main_window::MainWindow;

/// The classes of this namespace (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[MainView::TYPE, MainWindow::TYPE];

/// Makes the default constructors of the views reachable through their runtime types, which the
/// view locator creates them from ([`TypeInfo::register_constructor`]).
pub(crate) fn register_constructors() {
    TypeInfo::register_constructor::<MainView>();
    TypeInfo::register_constructor::<MainWindow>();
}

/// The classes of this namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&MainView::XAML_CLASS, &MainWindow::XAML_CLASS];
