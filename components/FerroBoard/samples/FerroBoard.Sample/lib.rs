//! Native sample composition; keyboard behavior belongs to the reusable library.
extern crate self as ferroboard_sample;
pub mod app;
pub mod bindings;
pub mod composition;
pub mod custom_layouts;
pub mod enum_choices;
pub mod language_bindings;
pub mod language_check_box;
pub mod languages;
pub mod main_view;
pub mod main_window;
pub mod native_command;
pub(crate) mod panel_context;
pub mod registration;
mod sample_data_paths;
pub mod sample_options;
pub mod sample_storage;
pub(crate) mod settings_callback_owner;
pub(crate) mod settings_field_view;
pub mod settings_fields;
pub mod settings_panel;
pub mod theme_choices;
pub mod view_model;
pub use app::App;
pub use main_view::MainView;
pub use main_window::MainWindow;
pub use registration::register_types;
pub use settings_panel::SettingsPanel;
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::*;
#[cfg(test)]
mod choice_tests;
#[cfg(test)]
mod options_tests;
#[cfg(test)]
mod resource_tests;
#[cfg(test)]
mod storage_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod language_tests;

#[cfg(test)]
mod panel_context_tests;

#[cfg(test)]
mod test_ui;

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod theme_tests;

#[cfg(test)]
mod host_scroll_tests;

#[cfg(test)]
mod wheel_routing_tests;

#[cfg(test)]
mod single_view_tests;
