//! Original ReactiveUI sample application, models and compiled views.
extern crate self as ferrodock_reactiveui_sample;
#[cfg(test)]
mod model_tests;
/// Original sample context model types.
pub mod models;
/// Original sample view models and layout factory.
pub mod view_models;

mod markup;
mod register_types;
pub use register_types::{register_types, ASSEMBLY};
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
#[cfg(test)]
pub(crate) use generated::compiled_xaml;
pub use generated::{compiled_dashboard_view, compiled_home_view, compiled_markup};

#[cfg(test)]
mod markup_tests;
/// Original compiled sample views.
pub mod views;
pub use generated::{
    compiled_dockable_options_view, compiled_document_view, compiled_tool1_view,
    compiled_tool2_view, compiled_tool3_view, compiled_tool4_view, compiled_tool5_view,
    compiled_tool6_view, compiled_tool7_view, compiled_tool8_view,
};
pub use generated::{compiled_main_view, compiled_main_window};
#[doc(hidden)]
pub use markup::render_options_metadata::RenderOptionsMetadata;
mod view_locator;
pub use view_locator::ViewLocator;
mod app;
pub use app::App;
/// Source application behaviors for passive views.
pub mod behaviors;
/// Framework-independent application theme-action contracts.
pub mod services;
pub use generated::compiled_app;
#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod layout_file_tests;
#[cfg(test)]
mod layout_persistence_tests;

mod infrastructure;
