//! `ColorView/` of the upstream project.

#[allow(clippy::module_inception)]
mod color_view;
mod color_view_properties;
mod color_view_tab;

pub use color_view::{ColorView, ColorViewImpl};
pub use color_view_tab::ColorViewTab;
