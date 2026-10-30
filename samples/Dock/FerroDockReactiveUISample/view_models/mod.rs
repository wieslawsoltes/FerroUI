mod dock_factory;
/// Original sample docks view models.
pub mod docks;
/// Original sample documents view models.
pub mod documents;
/// Original sample tools view models.
pub mod tools;
/// Original sample views view models.
pub mod views;
pub use dock_factory::DockFactory;
mod main_window_view_model;
pub use main_window_view_model::MainWindowViewModel;
mod main_view_theme_view_model;
pub use main_view_theme_view_model::MainViewThemeViewModel;
mod layout_file_view_model;
pub use layout_file_view_model::LayoutFileViewModel;
