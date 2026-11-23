//! Native FerroUI port of the pinned Microsoft WinUI Gallery catalog.
//!
//! The complete catalog and its original sample sources remain browsable. Executable
//! examples and migration diagnostics have separate states; browsing a page is never
//! represented as completing its WinUI behavior. See `README.md` and `port-status.json`.

#![forbid(unsafe_code)]

mod accessibility_samples;
mod assets;
mod assets_data;
mod binding_samples;
mod brush_samples;
mod button_samples;
mod cat_data;
mod chart_samples;
mod color_picker_samples;
mod command_samples;
mod connected_animation_samples;
pub mod catalog;
mod catalog_data;
pub mod contrast;
mod control_samples;
mod custom_controls;
mod date_samples;
pub mod examples;
mod feedback_samples;
mod frame_sample;
pub mod gallery;
mod ink_samples;
mod input_samples;
mod layout_samples;
mod list_samples;
mod materials_samples;
mod menu_samples;
mod media_samples;
pub mod playback_samples;
#[cfg(feature = "desktop")]
#[path = "../media_desktop.rs"]
mod desktop_media;
pub mod navigation;
mod motion_samples;
mod navigation_view_samples;
mod platform_samples;
mod repeater_samples;
mod scroll_samples;
mod shape_samples;
mod sizing_sample;
mod split_view_samples;
mod status_samples;
mod text_input_samples;
mod text_samples;
mod tree_samples;
mod window_samples;

#[cfg(any(feature = "desktop", feature = "capture"))]
mod app;

pub use catalog::{CatalogEntry, DocumentationLink, SampleDefinition, CATALOG, SOURCE_COMMIT};
pub use gallery::GalleryView;

#[cfg(any(feature = "desktop", feature = "capture"))]
pub use app::App;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod accessibility_samples_tests;

#[cfg(test)]
mod custom_controls_tests;

#[cfg(test)]
mod feedback_samples_tests;

#[cfg(test)]
mod motion_samples_tests;
#[cfg(test)]
mod navigation_view_samples_tests;

#[cfg(test)]
mod color_picker_samples_tests;

#[cfg(test)]
mod connected_animation_samples_tests;

#[cfg(test)]
mod media_samples_tests;

#[cfg(test)]
mod materials_samples_tests;

#[cfg(test)]
mod window_samples_tests;

#[cfg(test)]
mod text_samples_tests;

#[cfg(test)]
mod repeater_samples_tests;

#[cfg(test)]
mod ink_samples_tests;

#[cfg(test)]
mod chart_samples_tests;

#[cfg(test)]
mod x_bind_tests;

#[cfg(test)]
mod button_samples_tests;

#[cfg(test)]
mod layout_samples_tests;

#[cfg(test)]
mod text_input_samples_tests;

#[cfg(test)]
mod status_samples_tests;

#[cfg(test)]
mod command_samples_tests;

#[cfg(test)]
mod menu_samples_tests;

#[cfg(test)]
mod date_samples_tests;

#[cfg(test)]
mod list_samples_tests;

#[cfg(test)]
mod tree_samples_tests;

#[cfg(test)]
mod split_view_samples_tests;

#[cfg(test)]
mod shape_samples_tests;

#[cfg(test)]
mod scroll_samples_tests;

#[cfg(test)]
mod brush_samples_tests;

#[cfg(test)]
mod binding_samples_tests;
